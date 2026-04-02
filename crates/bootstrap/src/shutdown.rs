use snafu::ResultExt;

use crate::error;

/// Shuts down the system by triggering a clean power-off.
///
/// Uses `reboot(RB_AUTOBOOT)` to signal the kernel to shut down the system.
/// This should be called after the bootstrap process completes (shell exits
/// or supervise returns).
///
/// # Errors
///
/// Returns an error if the reboot syscall fails.
pub fn shutdown() -> Result<(), error::Error> {
    tracing::info!("Shutting down system");
    nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT)
        .context(error::ShutdownSnafu)?;
    Ok(())
}
