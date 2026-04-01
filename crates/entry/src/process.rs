use std::{
    ffi::CString,
    os::fd::{AsFd, OwnedFd},
};

use nix::{
    errno::Errno,
    fcntl::OFlag,
    sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, SigmaskHow, Signal},
    unistd::{self, ForkResult, Pid},
};
use snafu::ResultExt;

use crate::error::{self, Error};

/// A spawned child process with accessible output streams.
///
/// This struct represents a child process that has been spawned by
/// [`Process::spawn`]. It contains the process ID and file descriptors for
/// reading the child's stdout and stderr.
///
/// The stdout and stderr file descriptors are set to non-blocking mode and can
/// be used with `epoll` or similar I/O multiplexing mechanisms.
///
/// # Example
/// ```rust
/// use ocelot_entry::Process;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let process = Process::spawn("echo", ["hello".to_string()])?;
/// println!("Child PID: {}", process.pid);
/// # Ok(())
/// # }
/// ```
pub struct Process {
    /// The process ID of the child.
    pub pid: Pid,
    /// File descriptor for reading the child's standard output.
    ///
    /// This fd is opened in non-blocking mode and will return `EAGAIN` when
    /// no data is available.
    pub stdout_fd: OwnedFd,
    /// File descriptor for reading the child's standard error.
    ///
    /// This fd is opened in non-blocking mode and will return `EAGAIN` when
    /// no data is available.
    pub stderr_fd: OwnedFd,
}

impl Process {
    /// Spawns a child process with the given command and arguments.
    ///
    /// This function creates a new child process using `fork` and `execvp`,
    /// setting up pipes for stdout and stderr. The child's output can be
    /// read from the returned file descriptors.
    ///
    /// The command is passed directly to `execvp`, so it should be either
    /// a bare executable name (searched in PATH) or an absolute path.
    ///
    /// # Arguments
    ///
    /// * `command` - The executable to run. Must not contain interior null
    ///   bytes.
    /// * `args` - Iterator of arguments. The first argument should typically be
    ///   the command name (will be available as `argv[0]` in the child).
    ///
    /// # Returns
    ///
    /// Returns a [`Process`] struct containing:
    /// - The child's process ID (`pid`)
    /// - File descriptors for reading stdout and stderr
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidInput`] if the command or any argument contains
    /// interior null bytes.
    /// Returns [`Error::CreatePipe`] if creating a pipe fails.
    /// Returns [`Error::SpawnChild`] if forking fails.
    /// Returns [`Error::ReadPipe`] if reading from the error pipe fails
    /// (indicates child's exec failure).
    /// Returns [`Error::ChildExecute`] if the child process fails to execute
    /// the command.
    ///
    /// # Example
    /// ```rust
    /// # use ocelot_entry::Process;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let process = Process::spawn("echo", ["Hello".to_string(), "World".to_string()])?;
    /// println!("Spawned child with PID: {}", process.pid);
    /// # Ok(())
    /// # }
    /// ```
    pub fn spawn<Args>(command: &str, args: Args) -> Result<Self, Error>
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
        let (stdout_reader, stdout_writer) =
            unistd::pipe2(OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;
        let (stderr_reader, stderr_writer) =
            unistd::pipe2(OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;

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
                        Ok(Self { pid: child, stdout_fd: stdout_reader, stderr_fd: stderr_reader })
                    }
                    // Read 4 bytes.
                    // This means exec() failed and the child wrote the errno.
                    4 => {
                        let _errno = i32::from_ne_bytes(buf);
                        Err(Error::ExecuteChild)
                    }
                    _ => Err(Error::ExecuteChild),
                }
            }
            ForkResult::Child => {
                // Close the reader in child
                drop(err_reader);
                drop(stdout_reader);
                drop(stderr_reader);

                reset_signal_handling()?;

                unistd::dup2_stdout(&stdout_writer)
                    .map_err(|err| send_errno_and_exit(&err_writer, err));
                let _ = unistd::close(stdout_writer).ok();

                unistd::dup2_stderr(&stderr_writer)
                    .map_err(|err| send_errno_and_exit(&err_writer, err));
                let _ = unistd::close(stderr_writer).ok();

                match unistd::execvp(&c_cmd, &c_args) {
                    Ok(_) => unreachable!(
                        "The child process has created successfully and should not return from \
                         `execvp`"
                    ),
                    Err(error) => {
                        eprintln!(
                            "Failed to execute child process: {error}, with command: {command}"
                        );
                        send_errno_and_exit(&err_writer, error);
                    }
                }
            }
        }
    }
}

// Helper function to write errno to the error pipe and exit.
#[inline]
fn send_errno_and_exit(pipe_fd: &impl AsFd, errno: Errno) -> ! {
    let errno = std::io::Error::from(errno).raw_os_error().unwrap_or(1);
    let _ = unistd::write(pipe_fd, &errno.to_ne_bytes());

    #[expect(
        unsafe_code,
        reason = "Calling _exit after fork in child process is safe and necessary to exit without \
                  running destructors."
    )]
    unsafe {
        nix::libc::_exit(errno);
    }
}

fn reset_signal_handling() -> Result<(), Error> {
    let empty_set = SigSet::empty();
    signal::sigprocmask(SigmaskHow::SIG_SETMASK, Some(&empty_set), None)
        .context(error::SetSignalMaskSnafu)?;

    let default_handler = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());

    // Reset all signals to default handlers.
    // NOTE: SIGKILL and SIGSTOP cannot be caught or modified, so we skip them.
    // We ignore any errors from sigaction for other signals as they may not be
    // available on all platforms.
    let signals =
        Signal::iterator().filter(|&sig| sig != Signal::SIGSTOP && sig != Signal::SIGKILL);
    for signal in signals {
        #[expect(
            unsafe_code,
            reason = "Calling sigaction to reset signal handlers to default is safe in \
                      single-threaded context after fork"
        )]
        let _ = unsafe { signal::sigaction(signal, &default_handler) };
    }

    Ok(())
}
