/// Network configuration phase functions.
///
/// These functions configure network interfaces during the bootstrap process.
use crate::{config::NetworkConfig, error::Error};

/// Configures network before `switch_root`.
///
/// Currently a placeholder - pre-switch network configuration is not yet
/// implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn pre(_config: &NetworkConfig) -> Result<(), Error> {
    tracing::debug!("Pre-switch: network (not implemented)");
    Ok(())
}

/// Configures network after `switch_root`.
///
/// Currently a placeholder - post-switch network configuration is not yet
/// implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &NetworkConfig) -> Result<(), Error> {
    tracing::debug!("Post-switch: network (not implemented)");
    Ok(())
}
