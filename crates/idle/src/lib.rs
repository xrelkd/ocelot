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
        signal::{self, SigmaskHow, Signal},
        signalfd::{SigSet, SignalFd},
        wait::{self, WaitPidFlag, WaitStatus},
    },
    unistd,
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
/// * `Error::SetSignalMask` - Failed to block signals
/// * `Error::CreateSignalFd` - Failed to create signal file descriptor
/// * `Error::ParseSignal` - Received an invalid signal number
/// * `Error::ConvertU32` - Failed to convert signal number from u32 to i32
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
    let pid = unistd::getpid();
    if pid.as_raw() == 1 {
        tracing::info!("Start with PID 1");
    } else {
        tracing::warn!("Idle should be the first process (PID 1), current PID: {pid}");
    }

    // Block signals we want to handle and create a signal fd
    let signal_fd = {
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGINT);
        mask.add(Signal::SIGTERM);
        mask.add(Signal::SIGCHLD);
        signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None)
            .context(error::SetSignalMaskSnafu)?;
        SignalFd::new(&mask).context(error::CreateSignalFdSnafu)?
    };

    // Handle signals in a loop, especially SIGCHLD to reap child processes
    while let Ok(Some(sig_info)) = signal_fd.read_signal() {
        let sig = {
            let signo = sig_info.ssi_signo;
            let signal_num =
                i32::try_from(signo).with_context(|_| error::ConvertU32Snafu { value: signo })?;
            Signal::try_from(signal_num).with_context(|_| error::ParseSignalSnafu { signal_num })?
        };
        match sig {
            Signal::SIGCHLD => reap_zombies(),
            Signal::SIGINT | Signal::SIGTERM => {
                tracing::info!("Shutting down, got signal: {}", sig.as_str());
                break;
            }
            _ => {}
        }
    }

    reap_zombies();

    Ok(())
}

fn reap_zombies() {
    tracing::info!("Reaping any remaining zombie child processes...");
    let mut counter = 0;
    while let Ok(status) = wait::waitpid(None, Some(WaitPidFlag::WNOHANG)) {
        match status {
            WaitStatus::Exited(_pid, _code) => {
                counter += 1;
            }
            WaitStatus::Signaled(_pid, _sig, _) => {
                counter += 1;
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
