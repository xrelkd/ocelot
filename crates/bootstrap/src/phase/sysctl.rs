/// Sysctl configuration phase functions.
///
/// These functions configure kernel parameters via sysctl during the bootstrap
/// process.
use std::path::Path;

use snafu::ResultExt;

use crate::{
    config::Sysctl,
    error,
    error::{CreateDirectorySnafu, Error},
};

/// Configures sysctl parameters before `switch_root`.
///
/// Writes key-value pairs to `/proc/sys/` as kernel parameters.
pub fn pre(Sysctl { key_values }: &Sysctl) -> Result<(), Error> {
    for (key, value) in key_values {
        let sysctl_path = format!("/proc/sys/{}", key.replace('.', "/"));
        let path = Path::new(&sysctl_path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|_| CreateDirectorySnafu { path: parent.to_path_buf() })?;
        }

        std::fs::write(path, value.as_bytes()).with_context(|_| error::SetSysctlSnafu {
            path: path.to_path_buf(),
            value: value.clone(),
        })?;

        tracing::debug!("Pre-switch: set sysctl {key} = {value}");
    }
    Ok(())
}

/// Configures sysctl parameters after `switch_root`.
///
/// Currently a placeholder - post-switch sysctl configuration is not yet
/// implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &Sysctl) -> Result<(), Error> {
    tracing::debug!("Post-switch: sysctl (not implemented)");
    Ok(())
}
