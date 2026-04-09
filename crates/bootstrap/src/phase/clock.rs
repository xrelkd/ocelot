/// Clock configuration phase functions.
///
/// These functions configure system clock settings during the bootstrap
/// process.
use crate::{config::Clock, error::Error};

/// Configures clock settings before `switch_root`.
///
/// Currently a placeholder - RTC synchronization is not yet implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn pre(_config: &Clock) -> Result<(), Error> {
    tracing::debug!("Pre-switch: RTC sync (not implemented)");
    Ok(())
}

/// Configures clock settings after `switch_root`.
///
/// Currently a placeholder - clock configuration is not yet implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &Clock) -> Result<(), Error> {
    tracing::debug!("Post-switch: clock (not implemented)");
    Ok(())
}
