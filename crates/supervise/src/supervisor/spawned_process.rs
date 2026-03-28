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
    // RATIONALE: We use the trait only in our own code, or do not care about auto
    // traits like `Send` on the `Future`
    #[allow(async_fn_in_trait)]
    async fn spawn(&self) -> Result<SpawnedProcess, Error>;
}

impl CommandExt for Command {
    async fn spawn(&self) -> Result<SpawnedProcess, Error> {
        // Create a pipe with O_CLOEXEC
        // The pipe will automatically close on successful exec()
        let (reader_raw, writer_raw) =
            unistd::pipe2(OFlag::O_CLOEXEC | OFlag::O_NONBLOCK).context(error::CreatePipeSnafu)?;

        // SAFETY: We are calling `fork` in a way that is safe.
        #[allow(unsafe_code)]
        let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };

        match fork_result {
            ForkResult::Parent { child } => {
                // Close the writer in parent immediately
                drop(writer_raw);

                let reader = AsyncFd::new(reader_raw).context(error::ConvertAsyncFdSnafu)?;
                let _guard = reader.readable().await.expect("Pipe error");

                let mut buf = [0u8; 4];
                match unistd::read(reader.as_fd(), &mut buf).context(error::ReadPipeSnafu)? {
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
                drop(reader_raw);

                let err = self.exec();

                // If we are here, exec failed.
                // Write the errno to the pipe.
                let errno = err.raw_os_error().unwrap_or(1);
                let _ = unistd::write(&writer_raw, &errno.to_ne_bytes());

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
