//! Minimal init system and process supervisor for containerized environments.
//!
//! This crate provides functionality to spawn and manage a child process as a
//! minimal init system (PID 1 equivalent). It handles:
//! - Signal forwarding (SIGINT, SIGTERM) to the child
//! - Zombie process reaping (SIGCHLD)
//! - I/O multiplexing for stdout/stderr forwarding
//! - Optional timeout-based force-kill enforcement
//!
//! # Example
//!
//! ```
//! use std::time::Duration;
//! use ocelot_entry::{execute, Error};
//!
//! # fn main() -> Result<(), Error> {
//! // Run a simple command with a timeout
//! let exit_code = execute("true", [String::new()], Some(Duration::from_secs(5)))?;
//! println!("Child exited with code: {}", exit_code);
//! # Ok(())
//! # }
//! ```
//!
//! ## Design Notes
//!
//! - Uses `epoll` for efficient I/O multiplexing
//! - Uses `signalfd` to receive signals as readable events
//! - Implements non-blocking I/O with `splice` for zero-copy data forwarding
//! - Supports both blocking and non-blocking `waitpid` operations
//! - Works in container/PID namespace environments where PID 1 semantics are
//!   required

mod error;
mod process;
mod state;

use std::{os::fd::AsFd, time::Duration};

use nix::{
    sys::{
        epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
        signal::{self, SigmaskHow, Signal},
        signalfd::{SigSet, SignalFd},
        wait::{self, WaitPidFlag, WaitStatus},
    },
    unistd::{self, Pid},
};
use snafu::ResultExt;

pub use self::{error::Error, process::Process};
use crate::state::State;

const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_millis(100);
const DEFAULT_WAIT_TIMEOUT_AFTER_KILL: Duration = Duration::from_millis(200);

const SIGNAL_TOKEN: u64 = 0;
const CHILD_STDOUT_TOKEN: u64 = 1;
const CHILD_STDERR_TOKEN: u64 = 2;

/// Spawns a child process and manages its lifecycle as a minimal init system.
///
/// This function spawns a child process with the given command and arguments,
/// then acts as a process supervisor that:
/// - Forwards signals (SIGINT, SIGTERM) to the child process
/// - Reaps zombie processes (SIGCHLD)
/// - Optionally enforces a timeout after which the child is force-killed
///
/// The function blocks until the child process exits and returns its exit code.
///
/// # Arguments
///
/// * `command` - The command to execute. Must not contain interior null bytes.
/// * `args` - Iterator of arguments for the command. The first argument should
///   typically be the command name (will be available as `argv[0]`).
/// * `timeout` - Optional [`Duration`] specifying how long to wait after
///   sending a signal before force-killing with SIGKILL. If `None`, uses a
///   default of 100ms.
///
/// # Returns
///
/// Returns the exit code of the child process. If the child was terminated by a
/// signal, returns `128 + signal_number` (following Unix convention).
///
/// # Errors
///
/// Returns [`Error::SpawnChild`] if the `fork` system call fails or the child
/// fails to spawn due to invalid arguments or resource limits.
/// Returns [`Error::WaitPid`] if there's an error waiting for the child
/// process. Returns [`Error::ExecuteChild`] if the child's `execvp` call fails.
/// Returns [`Error::ReadPipe`] if reading from the status pipe fails.
/// Returns [`Error::CreatePipe`], [`Error::SetSignalMask`],
/// [`Error::CreateSignalFd`], [`Error::CreateEpoll`], [`Error::AddEpoll`],
/// [`Error::WaitEpoll`], or [`Error::ConvertTimeout`] for I/O setup failures.
///
/// # Signals
///
/// The supervisor sets up a signal mask that blocks SIGINT, SIGTERM, and
/// SIGCHLD and receives them via a signalfd. When received:
/// - SIGINT/SIGTERM: forwarded to the child process immediately
/// - SIGCHLD: triggers zombie reaping; if our child exited, we capture its
///   status
///
/// # Panics
///
/// This function should not panic under normal operation. It may panic only
/// if `tracing` initialization fails (unlikely) or if internal invariants are
/// violated (panic in child process during exec would indicate a bug in libc).
///
/// # Example
/// ```rust,no_run
/// # use std::time::Duration;
/// use ocelot_entry::{execute, Error};
///
/// # fn main() -> Result<(), Error> {
/// // Run a simple command with a 5-second timeout
/// let exit_code = execute("sleep", ["10".to_string()], Some(Duration::from_secs(5)))?;
/// println!("Child exited with code: {}", exit_code);
/// # Ok(())
/// # }
/// ```
pub fn execute<Command, Args>(
    command: Command,
    args: Args,
    timeout: Option<Duration>,
) -> Result<i32, Error>
where
    Command: Into<String>,
    Args: IntoIterator<Item = String>,
{
    check_pid();

    let Process { pid, stdout_fd: child_stdout_fd, stderr_fd: child_stderr_fd } =
        Process::spawn(&command.into(), args.into_iter())?;

    let mut state = State::new(pid, timeout.unwrap_or(DEFAULT_WAIT_TIMEOUT));

    let signal_fd = create_signal_fd()?;
    let epoll = create_epoll(&signal_fd, &child_stdout_fd, &child_stderr_fd)?;

    let mut events = [EpollEvent::empty(); 4];

    loop {
        // Check if the child process has exited before waiting for signals,
        // to avoid missing the exit status if it happens between signal checks.
        if !state.is_exited()
            && let Some(ReapedProcess { pid, exit_code }) = try_reap_process(state.id())?
        {
            tracing::info!("Reaped child process {pid} exited with status {exit_code}");
            state.set_exited(exit_code);
        }

        if state.is_exited() {
            break;
        }

        // Calculate the timeout for waiting on signals, and check if we need to force
        // kill the child process.
        if state.should_force_kill() {
            tracing::warn!(
                "Child process {pid} did not exit within the timeout, sending SIGKILL",
                pid = state.id()
            );
            if let Err(source) = signal::kill(state.id(), Signal::SIGKILL) {
                tracing::error!(
                    "Failed to send SIGKILL to child process {pid}: {source}",
                    pid = state.id()
                );
            }

            state.set_killed();
        }

        // Wait for a signal
        let wait_timeout = state.calculate_epoll_wait_timeout()?;
        let num_events = epoll.wait(&mut events, wait_timeout).context(error::WaitEpollSnafu)?;

        for event in events.iter().take(num_events) {
            match event.data() {
                SIGNAL_TOKEN => {
                    handle_signal(&signal_fd, &mut state)?;
                }
                CHILD_STDOUT_TOKEN => {
                    let binding = std::io::stdout();
                    let stdout = binding.as_fd();
                    let _eof = forward_data(&child_stdout_fd, &stdout);
                }
                CHILD_STDERR_TOKEN => {
                    let binding = std::io::stderr();
                    let stderr = binding.as_fd();
                    let _eof = forward_data(&child_stderr_fd, &stderr);
                }
                _ => {}
            }
        }
    }

    // Ensure the child process has exited, waiting if necessary
    if !state.is_exited()
        && let Ok(Some(ReapedProcess { exit_code, .. })) = try_reap_process_blocking(state.id())
    {
        state.set_exited(exit_code);
    }

    let (pid, status_code) = state.exited();
    tracing::info!("Child process {pid} exited with status {status_code}");

    let _ = reap_zombies();
    Ok(status_code)
}

/// Spawns an interactive shell with terminal setup, waits for it to exit.
///
/// This function creates a new session, sets up the console as controlling
/// terminal, forks and execs the shell, then waits for it to exit.
///
/// # Arguments
///
/// * `console` - Console device path (e.g., "tty1" or "/dev/tty1")
/// * `program` - Shell program to execute
/// * `args` - Arguments for the shell
/// * `timeout` - Optional duration to wait after signal before force-killing
///
/// # Returns
///
/// Returns the exit code of the shell process.
///
/// # Errors
///
/// Returns [`Error::CreatePipe`] if opening console device fails.
/// Returns [`Error::SpawnChild`] if forking fails.
///
/// # Panics
///
/// The child process may panic if dup2 or execv fails.
pub fn execute_interactive(
    program: &str,
    args: &[&str],
    console: &str,
    timeout: Option<Duration>,
) -> Result<i32, Error> {
    execute_interactive_with_session(program, args, console, true, timeout)
}

/// Spawns an interactive shell with terminal setup and optional session
/// creation, waits for it to exit.
///
/// This function sets up the console as controlling terminal, forks and execs
/// the shell, then waits for it to exit with proper signal handling and zombie
/// reaping.
///
/// # Arguments
///
/// * `console` - Console device path (e.g., "tty1" or "/dev/tty1")
/// * `program` - Shell program to execute
/// * `args` - Arguments for the shell
/// * `timeout` - Optional duration to wait after signal before force-killing
/// * `create_session` - If true, creates a new session for the shell child. Set
///   to false when the shell should inherit the parent's session (e.g., init
///   shell mode after `switch_root`).
///
/// # Returns
///
/// Returns the exit code of the shell process.
///
/// # Errors
///
/// Returns [`Error::CreatePipe`] if opening console device fails.
/// Returns [`Error::SpawnChild`] if forking fails.
///
/// # Panics
///
/// The child process may panic if dup2 or execv fails.
pub fn execute_interactive_with_session(
    program: &str,
    args: &[&str],
    console: &str,
    create_session: bool,
    timeout: Option<Duration>,
) -> Result<i32, Error> {
    let console_path =
        if console.starts_with('/') { console.to_string() } else { format!("/dev/{console}") };

    let mut state = {
        let console_file =
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&console_path)
                .with_context(|_| error::OpenConsoleSnafu { path: console_path.clone() })?;
        let pid = Process::spawn_with_console_and_session(
            &console_file,
            program,
            args.iter().map(|&s| s.to_string()),
            create_session,
        )?;
        State::new(pid, timeout.unwrap_or(DEFAULT_WAIT_TIMEOUT))
    };

    let signal_fd = create_signal_fd()?;

    loop {
        // Check if the child process has exited before waiting for signals,
        // to avoid missing the exit status if it happens between signal checks.
        if !state.is_exited()
            && let Some(ReapedProcess { pid, exit_code }) = try_reap_process(state.id())?
        {
            tracing::info!("Reaped child process {pid} exited with status {exit_code}");
            state.set_exited(exit_code);
        }

        if state.is_exited() {
            break;
        }

        // Calculate the timeout for waiting on signals, and check if we need to force
        // kill the child process.
        if state.should_force_kill() {
            tracing::warn!(
                "Child process {pid} did not exit within the timeout, sending SIGKILL",
                pid = state.id()
            );
            if let Err(source) = signal::kill(state.id(), Signal::SIGKILL) {
                tracing::error!(
                    "Failed to send SIGKILL to child process {pid}: {source}",
                    pid = state.id()
                );
            }

            state.set_killed();
        }

        // Handle signals in a loop, especially SIGCHLD to reap child processes
        while handle_signal(&signal_fd, &mut state).is_ok() {
            if state.is_exited() {
                break;
            }
        }
    }

    let (pid, status_code) = state.exited();
    tracing::info!("Child process {pid} exited with status {status_code}");

    let _ = reap_zombies();
    Ok(status_code)
}

fn check_pid() {
    let pid = unistd::getpid();
    if pid.as_raw() == 1 {
        tracing::info!("Start with PID 1");
    } else {
        tracing::warn!("Entry should be the first process (PID 1), current PID: {pid}");
    }
}

fn create_signal_fd() -> Result<SignalFd, Error> {
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGTERM);
    mask.add(Signal::SIGINT);
    mask.add(Signal::SIGCHLD);
    signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None)
        .context(error::SetSignalMaskSnafu)?;
    SignalFd::new(&mask).context(error::CreateSignalFdSnafu)
}

fn create_epoll(
    signal_fd: impl AsFd,
    child_stdout_fd: impl AsFd,
    child_stderr_fd: impl AsFd,
) -> Result<Epoll, Error> {
    let epoll = Epoll::new(EpollCreateFlags::empty()).context(error::CreateEpollSnafu)?;
    epoll
        .add(signal_fd, EpollEvent::new(EpollFlags::EPOLLIN, SIGNAL_TOKEN))
        .context(error::AddEpollSnafu)?;
    epoll
        .add(child_stdout_fd, EpollEvent::new(EpollFlags::EPOLLIN, CHILD_STDOUT_TOKEN))
        .context(error::AddEpollSnafu)?;
    epoll
        .add(child_stderr_fd, EpollEvent::new(EpollFlags::EPOLLIN, CHILD_STDERR_TOKEN))
        .context(error::AddEpollSnafu)?;
    Ok(epoll)
}

fn handle_signal(signal_fd: &SignalFd, state: &mut State) -> Result<(), Error> {
    let Ok(Some(sig_info)) = signal_fd.read_signal() else {
        return Ok(());
    };
    let sig = {
        let sig_num_i32 = i32::try_from(sig_info.ssi_signo)
            .context(error::ConvertSignalSnafu { value: sig_info.ssi_signo })?;
        Signal::try_from(sig_num_i32)
            .context(error::ParseSignalSnafu { signal_num: sig_num_i32 })?
    };

    match sig {
        Signal::SIGCHLD => {
            // Reap any child processes
            while let Some(ReapedProcess { pid, exit_code }) = try_reap_process(None)? {
                tracing::info!("Reaped child process (PID: {pid}) exited with status {exit_code}");
                if pid == state.id() {
                    state.set_exited(exit_code);
                }
            }
            Ok(())
        }
        Signal::SIGINT | Signal::SIGTERM => {
            tracing::info!(
                "Received signal {sig:?}, forwarding to child process {pid}",
                pid = state.id()
            );
            let _ = signal::kill(state.id(), sig).ok();
            state.set_signal_time();
            Ok(())
        }
        _ => Ok(()),
    }
}

struct ReapedProcess {
    pid: Pid,
    exit_code: i32,
}

/// Check the status of a child process without blocking.
fn try_reap_process<P: Into<Option<Pid>>>(pid: P) -> Result<Option<ReapedProcess>, Error> {
    match wait::waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
        Ok(WaitStatus::Exited(pid, exit_code)) => Ok(Some(ReapedProcess { pid, exit_code })),
        Ok(WaitStatus::Signaled(pid, sig, _)) => {
            Ok(Some(ReapedProcess { pid, exit_code: 128 + sig as i32 }))
        }
        Ok(_) | Err(nix::Error::ECHILD) => Ok(None),
        Err(source) => Err(Error::WaitPid { source }),
    }
}

fn try_reap_process_blocking(pid: Pid) -> Result<Option<ReapedProcess>, Error> {
    tracing::info!("Waiting for child process {pid} to exit...");
    let wait_status =
        wait::waitpid(pid, Some(WaitPidFlag::empty())).context(error::WaitPidSnafu)?;
    match wait_status {
        WaitStatus::Exited(pid, exit_code) => Ok(Some(ReapedProcess { pid, exit_code })),
        WaitStatus::Signaled(pid, sig, _) => {
            Ok(Some(ReapedProcess { pid, exit_code: 128 + sig as i32 }))
        }
        _ => Ok(None),
    }
}

#[inline]
#[must_use]
fn forward_data(source: &impl AsFd, destination: &impl AsFd) -> bool {
    // 128KiB
    const BATCH_SIZE: usize = 128 * 1024;
    let flags =
        nix::fcntl::SpliceFFlags::SPLICE_F_MOVE | nix::fcntl::SpliceFFlags::SPLICE_F_NONBLOCK;
    loop {
        match nix::fcntl::splice(source, None, destination, None, BATCH_SIZE, flags) {
            Ok(0) => return true,
            Ok(_) => {}
            Err(nix::Error::EAGAIN) => return false,
            Err(_) => return true,
        }
    }
}

fn reap_zombies() -> usize {
    tracing::info!("Reaping any remaining zombie child processes...");
    let mut counter = 0;
    while let Ok(status) = wait::waitpid(None, Some(WaitPidFlag::WNOHANG)) {
        match status {
            WaitStatus::Exited(pid, code) => {
                counter += 1;
                tracing::info!("Reaped child process {pid} with exit code {code}");
            }
            WaitStatus::Signaled(pid, sig, _) => {
                counter += 1;
                tracing::info!("Reaped child process {pid} terminated by signal {sig}");
            }
            _ => break,
        }
    }
    match counter {
        0 => {}
        1 => tracing::info!("Reaped 1 process"),
        n => tracing::info!("Reaped {n} processes"),
    }
    tracing::info!("Finished reaping child processes");
    counter
}
