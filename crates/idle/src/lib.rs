//! Minimalist PID 1 process for container environments.
//!
//! This crate provides a simple process supervisor designed to run as PID 1 in
//! containers. It handles signals (SIGINT, SIGTERM) for graceful shutdown and
//! automatically reaps zombie child processes via SIGCHLD handling.
//!
//! ## Organization
//!
//! - [`Error`]: Error type for signal handling failures
//! - [`execute`]: Main entry point that runs the idle loop

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

/// Runs the idle process loop, handling signals and reaping zombies.
///
/// This function blocks indefinitely, listening for three signals:
///
/// - **SIGCHLD**: Reaps any zombie child processes
/// - **SIGINT/SIGTERM**: Initiates graceful shutdown
///
/// Returns `Ok(())` on successful shutdown, or an error if signal handler
/// creation fails.
///
/// # Errors
///
/// * `Error::CreateSignalHandler` - Failed to create the signal handler,
///   typically due to insufficient permissions or resource limits
///
/// # Panics
///
/// This function never panics under normal operation.
///
/// # Signal Handling
///
/// When running as PID 1 in a container, the process receives termination
/// signals from the container runtime. Upon receiving SIGINT or SIGTERM,
/// the loop breaks and `execute()` returns, allowing the process to exit
/// cleanly.
///
/// ## Zombie Reaping
///
/// Child processes that exit become zombies until reaped by the parent.
/// This function continuously calls `waitpid(WNOHANG)` upon SIGCHLD to
/// reap all terminated children, preventing zombie accumulation.
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
