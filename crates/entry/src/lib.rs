mod error;

use std::{ffi::CString, sync::mpsc, thread::JoinHandle, time::Duration};

use nix::{
    sys::{
        signal,
        signal::Signal,
        wait::{self, WaitPidFlag, WaitStatus},
    },
    unistd,
    unistd::ForkResult,
};
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};
use snafu::ResultExt;

pub use self::error::Error;

const DEFAULT_WAIT_TIMEOUT: Duration = Duration::from_millis(100);
const DEFAULT_WAIT_TIMEOUT_AFTER_KILL: Duration = Duration::from_millis(200);

/// Spawns a child process and manages its lifecycle as a minimal init system.
///
/// This function spawns a child process with the given command and arguments,
/// then acts as a process supervisor that:
/// - Forwards signals (SIGINT, SIGTERM) to the child process
/// - Reaps zombie processes (SIGCHLD)
/// - Optionally enforces a timeout after which the child is force-killed
///
/// # Arguments
///
/// * `command` - The command to execute (converted to `String`)
/// * `args` - Iterator of arguments for the command (each converted to `OsStr`)
/// * `timeout` - Optional duration after which the child process will be killed
///   with SIGKILL
///
/// # Returns
///
/// Returns the exit code of the child process. If the child was terminated by a
/// signal, returns `128 + signal_number` (following Unix convention).
///
/// # Errors
///
/// Returns `Error::SpawnChild` if the child process fails to spawn (due to
/// invalid arguments).
/// Returns `Error::SpawnChildNix` if the fork fails.
/// Returns `Error::WaitPid` if there's an error waiting for the child process.
///
/// # Panics
///
/// This function should not panic under normal operation.
pub fn execute<Command, Args>(
    command: Command,
    args: Args,
    timeout: Option<Duration>,
) -> Result<i32, Error>
where
    Command: Into<String>,
    Args: IntoIterator<Item = String>,
{
    let pid = unistd::getpid();
    if pid.as_raw() != 1 {
        tracing::warn!("Entry should be the first process (PID 1), current PID: {pid}");
    }

    let child_pid = fork_and_spawn_child(&command.into(), args.into_iter())?;
    let (spawned_signal_thread, signal_rx) = SpawnedSignalThread::new()?;

    let mut signal_time = None::<std::time::Instant>;
    let mut child_exited = false;
    let mut child_status = 0;

    loop {
        // Check if the child process has exited before waiting for signals, to avoid
        // missing the exit status if it happens between signal checks.
        if !child_exited
            && let Some(ReapedProcess { pid, exit_code }) = check_child_status(child_pid)?
        {
            tracing::info!("Reaped child process {pid} exited with status {exit_code}");
            child_exited = true;
            child_status = exit_code;
        }

        if child_exited {
            break;
        }

        // Calculate the timeout for waiting on signals, and check if we need to force
        // kill the child process.
        let CalculateWaitTimeout { should_force_kill, wait_timeout } =
            calculate_wait_timeout(signal_time, timeout);
        if should_force_kill {
            tracing::warn!(
                "Child process {child_pid} did not exit within the timeout, sending SIGKILL"
            );
            if let Err(source) = signal::kill(child_pid, Signal::SIGKILL) {
                tracing::error!("Failed to send SIGKILL to child process {child_pid}: {source}");
            }

            // Sleep briefly to allow the `SIGKILL` to take effect before checking for the
            // child's exit status again.
            std::thread::sleep(DEFAULT_WAIT_TIMEOUT_AFTER_KILL);

            // Break here to check for the child's exit status immediately after sleeping
            // for a while, rather than waiting for the next signal. This ensures we don't
            // miss the child's exit if it happens right after the timeout.
            break;
        }

        // Wait for a signal
        match signal_rx.recv_timeout(wait_timeout) {
            Ok(SIGCHLD) => {
                // Attempt to reap any child processes.
                while let Some(ReapedProcess { pid, exit_code }) = check_child_status(None)? {
                    tracing::info!("Reaped child process {pid} exited with status {exit_code}");
                    if pid == child_pid {
                        child_exited = true;
                        child_status = exit_code;
                    }
                }
            }
            Ok(sig @ (SIGINT | SIGTERM)) => {
                if signal_time.is_none() {
                    signal_time = Some(std::time::Instant::now());
                }
                let sig = Signal::try_from(sig).expect("valid signal");
                tracing::info!("Received signal {sig}, forwarding to child process {child_pid}");
                let _ = signal::kill(child_pid, sig).ok();
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            _ => {}
        }
    }

    spawned_signal_thread.close();

    // Ensure the child process has exited, waiting if necessary
    if !child_exited
        && let Ok(Some(ReapedProcess { exit_code, .. })) = wait_child_blocking(child_pid)
    {
        child_status = exit_code;
    }
    tracing::info!("Child process {child_pid} exited with status {child_status}");

    reap_zombies();
    Ok(child_status)
}

struct ReapedProcess {
    pid: unistd::Pid,
    exit_code: i32,
}

/// Check the status of a child process without blocking. Returns `Some((pid,
/// exit_code))` if the child has exited or was signaled, or `None` if the child
/// is still running or there are no child processes.
fn check_child_status<P: Into<Option<unistd::Pid>>>(
    pid: P,
) -> Result<Option<ReapedProcess>, Error> {
    match wait::waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
        Ok(WaitStatus::Exited(pid, exit_code)) => Ok(Some(ReapedProcess { pid, exit_code })),
        Ok(WaitStatus::Signaled(pid, sig, _)) => {
            Ok(Some(ReapedProcess { pid, exit_code: 128 + sig as i32 }))
        }
        Ok(_) | Err(nix::Error::ECHILD) => Ok(None),
        Err(source) => Err(Error::WaitPid { source }),
    }
}

fn wait_child_blocking(pid: unistd::Pid) -> Result<Option<ReapedProcess>, Error> {
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

fn reap_zombies() {
    tracing::info!("Reaping any remaining zombie child processes...");
    while let Ok(status) = wait::waitpid(None, Some(WaitPidFlag::WNOHANG)) {
        match status {
            WaitStatus::Exited(pid, code) => {
                tracing::info!("Reaped child process {pid} with exit code {code}");
            }
            WaitStatus::Signaled(pid, sig, _) => {
                tracing::info!("Reaped child process {pid} terminated by signal {sig}");
            }
            _ => break,
        }
    }
    tracing::info!("Finished reaping child processes");
}

fn fork_and_spawn_child<Args>(command: &str, args: Args) -> Result<unistd::Pid, Error>
where
    Args: IntoIterator<Item = String>,
{
    let c_cmd = CString::new(command)
        .with_context(|_| error::InvalidInputSnafu { input: command.to_string() })?;

    let c_args = std::iter::once(Ok(c_cmd.clone()))
        .chain(args.into_iter().map(|arg| {
            CString::new(arg.clone()).with_context(|_| error::InvalidInputSnafu { input: arg })
        }))
        .collect::<Result<Vec<_>, Error>>()?;

    tracing::info!("Spawning child process with {c_args:?}");

    // SAFETY: We are calling `fork` in a way that is safe.
    #[allow(unsafe_code)]
    let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };
    match fork_result {
        ForkResult::Parent { child } => Ok(child),
        ForkResult::Child => match unistd::execvp(&c_cmd, &c_args) {
            Ok(_) => unreachable!(
                "The child process has created successfully and should not return from `execvp`"
            ),
            Err(error) => {
                eprintln!("Failed to execute child process: {error}, with command: {command}");
                std::process::exit(1);
            }
        },
    }
}

struct CalculateWaitTimeout {
    should_force_kill: bool,
    wait_timeout: Duration,
}

fn calculate_wait_timeout(
    signal_time: Option<std::time::Instant>,
    timeout: Option<Duration>,
) -> CalculateWaitTimeout {
    if let Some(sig_time) = signal_time
        && let Some(timeout) = timeout
    {
        let elapsed = sig_time.elapsed();
        if elapsed >= timeout {
            CalculateWaitTimeout {
                should_force_kill: true,
                wait_timeout: DEFAULT_WAIT_TIMEOUT_AFTER_KILL,
            }
        } else {
            let wait_timeout = timeout
                .checked_sub(elapsed)
                .unwrap_or(DEFAULT_WAIT_TIMEOUT)
                .min(DEFAULT_WAIT_TIMEOUT);
            CalculateWaitTimeout { should_force_kill: false, wait_timeout }
        }
    } else {
        CalculateWaitTimeout { should_force_kill: false, wait_timeout: DEFAULT_WAIT_TIMEOUT }
    }
}

struct SpawnedSignalThread {
    thread: Option<JoinHandle<()>>,
    signals_handle: signal_hook::iterator::backend::Handle,
}

impl SpawnedSignalThread {
    pub fn new() -> Result<(Self, mpsc::Receiver<i32>), Error> {
        let mut signals =
            Signals::new([SIGINT, SIGTERM, SIGCHLD]).context(error::CreateSignalHandlerSnafu)?;
        let signals_handle = signals.handle();
        let (tx, signal_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            for sig in signals.forever() {
                let _ = tx.send(sig).ok();
            }
        });
        Ok((Self { thread: Some(thread), signals_handle }, signal_rx))
    }

    pub fn close(self) { drop(self); }
}

impl Drop for SpawnedSignalThread {
    fn drop(&mut self) {
        self.signals_handle.close();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join().ok();
        }
    }
}
