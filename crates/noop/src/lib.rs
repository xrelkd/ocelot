mod error;

use nix::{
    sys::{
        signal::Signal,
        wait::{WaitPidFlag, waitpid},
    },
    unistd::getpid,
};
use signal_hook::{
    consts::{SIGCHLD, SIGINT, SIGTERM},
    iterator::Signals,
};
use snafu::ResultExt;

pub use self::error::Error;

/// A simple process that waits indefinitely, handling signals to reap child
/// processes and shut down gracefully.
///
/// # Errors
/// This function returns an error if it fails to create the signal handler.
///
/// # Panics
/// This function never panics.
pub fn execute() -> Result<(), Error> {
    // Get the PID and warn if not running as PID 1
    let pid = getpid();
    if pid.as_raw() != 1 {
        tracing::warn!("Pause should be the first process (PID 1), current PID: {pid}");
    }

    let mut signals =
        Signals::new([SIGINT, SIGTERM, SIGCHLD]).context(error::CreateSignalHandlerSnafu)?;

    // Handle signals in a loop, especially SIGCHLD to reap child processes
    for sig in signals.forever() {
        match sig {
            SIGCHLD => {
                let options = Some(WaitPidFlag::WNOHANG);
                while let Ok(status) = waitpid(None, options)
                    && status.pid().is_some()
                {}
            }
            SIGINT | SIGTERM => {
                let sig = Signal::try_from(sig).expect("`sig` is valid");
                let sig = sig.as_str();
                tracing::info!("Shutting down, got signal: {sig}");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
