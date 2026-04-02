use std::os::fd::{AsFd, OwnedFd};

use nix::{
    fcntl::OFlag,
    sys::stat::Mode,
    unistd::{self, ForkResult, Pid},
};
use snafu::ResultExt;
use tokio::io::unix::AsyncFd;

use crate::{Error, command::Command, error};

#[derive(Debug)]
pub struct SpawnedProcess {
    pub pid: Pid,
    pub pgid: Pid,
    pub stdout_fd: Option<OwnedFd>,
    pub stderr_fd: Option<OwnedFd>,
}

impl AsRef<Pid> for SpawnedProcess {
    fn as_ref(&self) -> &Pid { &self.pid }
}

pub trait CommandExt {
    async fn spawn(&self) -> Result<SpawnedProcess, Error>;
}

impl CommandExt for Command {
    async fn spawn(&self) -> Result<SpawnedProcess, Error> {
        // Create pipes only if we're not discarding stdout/stderr
        let (stdout_reader, stdout_writer) = if self.is_discard_stdout() {
            (None, None)
        } else {
            let (r, w) = unistd::pipe2(OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;
            (Some(r), Some(w))
        };
        let (stderr_reader, stderr_writer) = if self.is_discard_stderr() {
            (None, None)
        } else {
            let (r, w) = unistd::pipe2(OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;
            (Some(r), Some(w))
        };

        // Error pipe for child exec failure reporting
        let (err_reader, err_writer) =
            unistd::pipe2(OFlag::O_CLOEXEC).context(error::CreatePipeSnafu)?;

        #[expect(unsafe_code, reason = "We are calling `fork` in a way that is safe.")]
        let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };

        match fork_result {
            ForkResult::Parent { child } => {
                drop(err_writer);
                drop(stdout_writer);
                drop(stderr_writer);

                let err_reader = AsyncFd::new(err_reader).context(error::RegisterFdSnafu)?;
                let _guard = err_reader.readable().await.context(error::ReadPipeSnafu)?;

                let mut buf = [0u8; 4];
                match unistd::read(err_reader.as_fd(), &mut buf)
                    .map_err(std::io::Error::from)
                    .context(error::ReadPipeSnafu)?
                {
                    0 => Ok(SpawnedProcess {
                        pid: child,
                        pgid: child,
                        stdout_fd: stdout_reader,
                        stderr_fd: stderr_reader,
                    }),
                    4 => {
                        let _errno = i32::from_ne_bytes(buf);
                        Err(Error::ExecuteChild)
                    }
                    _ => Err(Error::ExecuteChild),
                }
            }
            ForkResult::Child => {
                drop(err_reader);
                drop(stdout_reader);
                drop(stderr_reader);

                // Place this process in its own process group so that shutdown signals
                // can be sent to the entire group (including any child processes spawned
                // by the supervised process, e.g., sshd sessions).
                if let Err(err) = unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0)) {
                    // If setpgid fails, we still proceed — the process will share the
                    // parent's process group but shutdown will still work (just less
                    // thorough for multi-process children).
                    eprintln!("Failed to set process group: {err}");
                }

                // Handle stdout: redirect to pipe writer or /dev/null
                if let Some(writer) = stdout_writer {
                    unistd::dup2_stdout(&writer)
                        .map_err(|err| send_errno_and_exit(&err_writer, err));
                    let _ = unistd::close(writer);
                } else {
                    redirect_to_dev_null(&err_writer, |fd| unistd::dup2_stdout(fd));
                }

                // Handle stderr: redirect to pipe writer or /dev/null
                if let Some(writer) = stderr_writer {
                    unistd::dup2_stderr(&writer)
                        .map_err(|err| send_errno_and_exit(&err_writer, err));
                    let _ = unistd::close(writer);
                } else {
                    redirect_to_dev_null(&err_writer, |fd| unistd::dup2_stderr(fd));
                }

                // Execute the command; on failure, write errno and exit
                let err = self.exec();
                send_errno_and_exit(&err_writer, err);
            }
        }
    }
}

// Helper function to write errno to the error pipe and exit.
#[inline]
fn send_errno_and_exit(error_writer: &impl AsFd, error: impl Into<std::io::Error>) -> ! {
    let error = error.into();
    let errno = error.raw_os_error().unwrap_or(1);
    let _ = unistd::write(error_writer, &errno.to_ne_bytes());

    #[expect(
        unsafe_code,
        reason = "Calling _exit after fork in child process is safe and necessary to exit without \
                  running destructors."
    )]
    unsafe {
        nix::libc::_exit(errno);
    }
}

/// Helper to redirect a file descriptor to `/dev/null` in the child process.
#[inline]
fn redirect_to_dev_null(
    err_writer: &impl AsFd,
    redirect_fn: impl FnOnce(&OwnedFd) -> Result<(), nix::Error>,
) {
    let dev_null =
        match nix::fcntl::open("/dev/null", OFlag::O_WRONLY | OFlag::O_CLOEXEC, Mode::empty()) {
            Ok(fd) => fd,
            Err(err) => send_errno_and_exit(err_writer, err),
        };
    redirect_fn(&dev_null).map_err(|err| send_errno_and_exit(err_writer, err));
    let _ = unistd::close(dev_null);
}

#[cfg(test)]
mod tests {
    use crate::{command::Command, supervisor::spawned_process::CommandExt};

    #[tokio::test]
    async fn test_spawn_success() {
        let result = Command::new("true").spawn().await;
        assert!(result.is_ok());
        let spawned = result.unwrap();
        assert!(spawned.pid.as_raw() > 0);
        assert_eq!(spawned.pgid, spawned.pid);
    }

    #[tokio::test]
    async fn test_spawn_failure() {
        let result = Command::new("no-this-command-should-fail").spawn().await;
        assert!(result.is_err());
    }
}
