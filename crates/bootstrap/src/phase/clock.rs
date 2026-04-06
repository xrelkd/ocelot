use crate::{config::Clock, error::Error};

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn pre(_config: &Clock) -> Result<(), Error> {
    tracing::debug!("Pre-switch: RTC sync (not implemented)");
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &Clock) -> Result<(), Error> {
    tracing::debug!("Post-switch: clock (not implemented)");
    Ok(())
}
