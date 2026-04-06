use crate::{config::NetworkConfig, error::Error};

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn pre(_config: &NetworkConfig) -> Result<(), Error> {
    tracing::debug!("Pre-switch: network (not implemented)");
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &NetworkConfig) -> Result<(), Error> {
    tracing::debug!("Post-switch: network (not implemented)");
    Ok(())
}
