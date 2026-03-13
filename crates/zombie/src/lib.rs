mod error;

use std::{sync::mpsc, time::Duration};

use nix::{sys::signal::Signal, unistd, unistd::ForkResult};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use snafu::ResultExt;

pub use self::error::Error;

/// A deliberate zombie process generator for Unix-like systems.
///
/// # Errors
/// This function returns an error if it fails to create the signal handler.
///
/// # Panics
/// This function never panics.
pub fn execute(interval: Duration, zombie_limit: Option<u64>) -> Result<(), Error> {
    let mut signals = Signals::new([SIGINT, SIGTERM]).context(error::CreateSignalHandlerSnafu)?;
    let (signal_tx, signal_rx) = mpsc::channel();
    let handle = signals.handle();
    let thread = std::thread::spawn(move || {
        if let Some(sig) = signals.forever().next() {
            let sig = Signal::try_from(sig).expect("`sig` is valid");
            let sig = sig.as_str();
            let _ = signal_tx.send(sig).ok();
        }
    });

    let pid = unistd::getpid();
    tracing::info!("[Parent] PID: {pid}");

    let mut zombie_count = 0;
    loop {
        if Some(zombie_count) >= zombie_limit {
            tracing::info!("[Parent] Zombie limit reached, exiting parent process {pid}");
            break;
        }
        zombie_count += 1;

        // SAFETY: We are calling `fork` in a way that is safe.
        #[allow(unsafe_code)]
        let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };
        match fork_result {
            ForkResult::Parent { child } => {
                tracing::info!("[Parent] Spawned child PID: {child}, zombie: {zombie_count}");
            }
            ForkResult::Child => {
                // Child process immediately exits, creating a zombie in the parent process
                let self_pid = unistd::getpid();
                tracing::info!("[Child {self_pid}] Exited");
                return Ok(());
            }
        }

        match signal_rx.recv_timeout(interval) {
            Ok(sig) => {
                tracing::info!("Shutting down, got signal: {sig}");
                break;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout occurred, continue to spawn zombies
            }
        }
    }

    if !handle.is_closed() {
        handle.close();
    }
    let _ = thread.join().ok();

    Ok(())
}
