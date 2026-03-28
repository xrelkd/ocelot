mod error;

use std::{
    ffi::CString,
    io::PipeReader,
    os::fd::{FromRawFd, IntoRawFd},
    sync::mpsc,
    thread::JoinHandle,
    time::Duration,
};

use nix::{
    fcntl::OFlag,
    sys::{
        signal,
        signal::Signal,
        wait,
        wait::{WaitPidFlag, WaitStatus},
    },
    unistd,
    unistd::{ForkResult, Pid},
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
    if pid.as_raw() == 1 {
        tracing::info!("Start with PID 1");
    } else {
        tracing::warn!("Entry should be the first process (PID 1), current PID: {pid}");
    }

    let Process { pid, stdout: mut child_stdout, stderr: mut child_stderr } =
        Process::spawn(&command.into(), args.into_iter())?;

    let stdout_thread = std::thread::spawn(move || {
        let mut stdout_sink = std::io::stdout();
        if let Err(err) = std::io::copy(&mut child_stdout, &mut stdout_sink) {
            tracing::error!("Error forwarding logs: {err}");
        }
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut stderr_sink = std::io::stderr();
        if let Err(err) = std::io::copy(&mut child_stderr, &mut stderr_sink) {
            tracing::error!("Error forwarding logs: {err}");
        }
    });

    let (spawned_signal_thread, signal_rx) = SpawnedSignalThread::new()?;

    let mut state = ExecutionState::new(pid, timeout);

    loop {
        // Check if the child process has exited before waiting for signals,
        // to avoid missing the exit status if it happens between signal checks.
        if !state.process_exited
            && let Some(ReapedProcess { pid, exit_code }) = check_child_status(state.pid)?
        {
            tracing::info!("Reaped child process {pid} exited with status {exit_code}");
            state.set_exited(exit_code);
        }

        if state.process_exited {
            break;
        }

        // Calculate the timeout for waiting on signals, and check if we need to force
        // kill the child process.
        if state.should_force_kill() {
            tracing::warn!(
                "Child process {pid} did not exit within the timeout, sending SIGKILL",
                pid = state.pid
            );
            if let Err(source) = signal::kill(state.pid, Signal::SIGKILL) {
                tracing::error!(
                    "Failed to send SIGKILL to child process {pid}: {source}",
                    pid = state.pid
                );
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
        let wait_timeout = state.calculate_wait_timeout();
        let should_break = handle_signal(&signal_rx, wait_timeout, &mut state)?;
        if should_break {
            break;
        }
    }

    spawned_signal_thread.close();
    let _unused = stdout_thread.join();
    let _unused = stderr_thread.join();

    // Ensure the child process has exited, waiting if necessary
    if !state.process_exited
        && let Ok(Some(ReapedProcess { exit_code, .. })) = wait_child_blocking(state.pid)
    {
        state.set_exited(exit_code);
    }
    tracing::info!(
        "Child process {pid} exited with status {status}",
        pid = state.pid,
        status = state.status_code
    );

    reap_zombies();
    Ok(state.status_code)
}

struct ExecutionState {
    pid: Pid,
    process_exited: bool,
    status_code: i32,
    signal_time: Option<std::time::Instant>,
    timeout: Option<Duration>,
}

impl ExecutionState {
    const fn new(pid: Pid, timeout: Option<Duration>) -> Self {
        Self { pid, signal_time: None, process_exited: false, status_code: 0, timeout }
    }

    const fn set_exited(&mut self, status_code: i32) {
        self.status_code = status_code;
        self.process_exited = true;
    }

    fn should_force_kill(&self) -> bool {
        match (self.signal_time, self.timeout) {
            (Some(sig_time), Some(timeout)) => sig_time.elapsed() >= timeout,
            _ => false,
        }
    }

    fn calculate_wait_timeout(&self) -> Duration {
        match (self.signal_time, self.timeout) {
            (Some(sig_time), Some(timeout)) => {
                let elapsed = sig_time.elapsed();
                if elapsed >= timeout {
                    DEFAULT_WAIT_TIMEOUT_AFTER_KILL
                } else {
                    timeout
                        .checked_sub(elapsed)
                        .unwrap_or(DEFAULT_WAIT_TIMEOUT)
                        .min(DEFAULT_WAIT_TIMEOUT)
                }
            }
            _ => DEFAULT_WAIT_TIMEOUT,
        }
    }
}

/// Handles signals from the signal thread.
/// Returns `Ok(true)` if the loop should break (e.g., signal thread
/// disconnected). Returns `Ok(false)` to continue looping.
fn handle_signal(
    signal_rx: &mpsc::Receiver<i32>,
    wait_timeout: Duration,
    state: &mut ExecutionState,
) -> Result<bool, Error> {
    match signal_rx.recv_timeout(wait_timeout) {
        Ok(SIGCHLD) => {
            // Reap any child processes
            while let Some(ReapedProcess { pid, exit_code }) = check_child_status(None)? {
                tracing::info!("Reaped child process (PID: {pid}) exited with status {exit_code}");
                if pid == state.pid {
                    state.set_exited(exit_code);
                }
            }
            Ok(false)
        }
        Ok(sig @ (SIGINT | SIGTERM)) => {
            if state.signal_time.is_none() {
                state.signal_time = Some(std::time::Instant::now());
            }
            let sig = Signal::try_from(sig).expect("SIGINT or SIGTERM are always valid");
            tracing::info!(
                "Received signal {sig:?}, forwarding to child process {pid}",
                pid = state.pid
            );
            let _ = signal::kill(state.pid, sig).ok();
            Ok(false)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => Ok(true),
        _ => Ok(false),
    }
}

struct ReapedProcess {
    pid: Pid,
    exit_code: i32,
}

/// Check the status of a child process without blocking.
fn check_child_status<P: Into<Option<Pid>>>(pid: P) -> Result<Option<ReapedProcess>, Error> {
    match wait::waitpid(pid, Some(WaitPidFlag::WNOHANG)) {
        Ok(WaitStatus::Exited(pid, exit_code)) => Ok(Some(ReapedProcess { pid, exit_code })),
        Ok(WaitStatus::Signaled(pid, sig, _)) => {
            Ok(Some(ReapedProcess { pid, exit_code: 128 + sig as i32 }))
        }
        Ok(_) | Err(nix::Error::ECHILD) => Ok(None),
        Err(source) => Err(Error::WaitPid { source }),
    }
}

fn wait_child_blocking(pid: Pid) -> Result<Option<ReapedProcess>, Error> {
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
}

struct Process {
    pid: Pid,
    stdout: PipeReader,
    stderr: PipeReader,
}

impl Process {
    fn spawn<Args>(command: &str, args: Args) -> Result<Self, Error>
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

        // Create pipes for handling stdout/stderr.
        let (stdout_reader, stdout_writer) = unistd::pipe().context(error::CreatePipeSnafu)?;
        let (stderr_reader, stderr_writer) = unistd::pipe().context(error::CreatePipeSnafu)?;

        // Create a pipe with `O_CLOEXEC`.
        // The pipe will automatically close on successful `exec()`.
        let (err_reader, err_writer) =
            unistd::pipe2(OFlag::O_CLOEXEC).context(error::CreatePipeSnafu)?;

        #[expect(unsafe_code, reason = "We are calling `fork` in a way that is safe.")]
        let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };

        match fork_result {
            ForkResult::Parent { child } => {
                // Close the writer in parent immediately
                drop(err_writer);
                drop(stdout_writer);
                drop(stderr_writer);

                let mut buf = [0u8; 4];
                match unistd::read(err_reader, &mut buf).context(error::ReadPipeSnafu)? {
                    // Read 0 bytes (EOF).
                    // This means the child successfully called exec() and the pipe closed.
                    0 => {
                        #[expect(
                            unsafe_code,
                            reason = "We need to encapsulate the Pipe reader from a raw fd"
                        )]
                        let stdout_reader =
                            unsafe { PipeReader::from_raw_fd(stdout_reader.into_raw_fd()) };
                        #[expect(
                            unsafe_code,
                            reason = "We need to encapsulate the Pipe reader from a raw fd"
                        )]
                        let stderr_reader =
                            unsafe { PipeReader::from_raw_fd(stderr_reader.into_raw_fd()) };
                        Ok(Self { pid: child, stdout: stdout_reader, stderr: stderr_reader })
                    }
                    // Read 4 bytes.
                    // This means exec() failed and the child wrote the errno.
                    4 => {
                        let _errno = i32::from_ne_bytes(buf);
                        Err(Error::ChildExecute)
                    }
                    _ => Err(Error::ChildExecute),
                }
            }
            ForkResult::Child => {
                // Close the reader in child
                drop(err_reader);
                drop(stdout_reader);
                drop(stderr_reader);

                if let Err(err) = unistd::dup2_stdout(&stdout_writer) {
                    let errno = std::io::Error::from(err).raw_os_error().unwrap_or(1);
                    let _ = unistd::write(&err_writer, &errno.to_ne_bytes());
                    std::process::exit(errno);
                }
                let _ = unistd::close(stdout_writer).ok();

                if let Err(err) = unistd::dup2_stderr(&stderr_writer) {
                    let errno = std::io::Error::from(err).raw_os_error().unwrap_or(1);
                    let _ = unistd::write(&err_writer, &errno.to_ne_bytes());
                    std::process::exit(errno);
                }
                let _ = unistd::close(stderr_writer).ok();

                match unistd::execvp(&c_cmd, &c_args) {
                    Ok(_) => unreachable!(
                        "The child process has created successfully and should not return from \
                         `execvp`"
                    ),
                    Err(error) => {
                        // If we are here, exec failed.
                        eprintln!(
                            "Failed to execute child process: {error}, with command: {command}"
                        );

                        // Write the errno to the pipe.
                        let errno = std::io::Error::from(error).raw_os_error().unwrap_or(1);
                        let _ = unistd::write(&err_writer, &errno.to_ne_bytes());

                        std::process::exit(errno);
                    }
                }
            }
        }
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
