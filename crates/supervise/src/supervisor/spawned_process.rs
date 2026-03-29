use std::os::fd::AsFd;

use nix::{
    fcntl::OFlag,
    unistd,
    unistd::{ForkResult, Pid},
};
use snafu::ResultExt;
use tokio::io::unix::AsyncFd;

use crate::{Error, command::Command, error};

#[derive(Debug)]
pub struct SpawnedProcess {
    pub pid: Pid,
}

impl AsRef<Pid> for SpawnedProcess {
    fn as_ref(&self) -> &Pid { &self.pid }
}

pub trait CommandExt {
    async fn spawn(&self) -> Result<SpawnedProcess, Error>;
}

impl CommandExt for Command {
    async fn spawn(&self) -> Result<SpawnedProcess, Error> {
        // Create a pipe with O_CLOEXEC
        // The pipe will automatically close on successful exec()
        let (err_reader, err_writer) =
            unistd::pipe2(OFlag::O_CLOEXEC | OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;

        #[expect(unsafe_code, reason = "We are calling `fork` in a way that is safe.")]
        let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };

        match fork_result {
            ForkResult::Parent { child } => {
                // Close the writer in parent immediately
                drop(err_writer);

                let err_reader = AsyncFd::new(err_reader).context(error::RegisterFdSnafu)?;
                let _guard = err_reader.readable().await.expect("Pipe error");

                let mut buf = [0u8; 4];
                match unistd::read(err_reader.as_fd(), &mut buf).context(error::ReadPipeSnafu)? {
                    // Case A: Read 0 bytes (EOF).
                    // This means the child successfully called exec() and the pipe closed.
                    0 => Ok(SpawnedProcess { pid: child }),

                    // Case B: Read 4 bytes.
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

                let err = self.exec();

                // If we are here, exec failed.
                // Write the errno to the pipe.
                let errno = err.raw_os_error().unwrap_or(1);
                let _ = unistd::write(&err_writer, &errno.to_ne_bytes());

                std::process::exit(errno);
            }
        }
    }
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
    }

    #[tokio::test]
    async fn test_spawn_failure() {
        let result = Command::new("no-this-command-should-fail").spawn().await;
        assert!(result.is_err());
    }
}
