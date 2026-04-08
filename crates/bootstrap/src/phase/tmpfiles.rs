/// Tmpfile creation phase functions.
///
/// These functions create temporary files during the bootstrap process.
use std::{os::unix::fs::PermissionsExt, path::Path};

use snafu::ResultExt;

use crate::{
    config::Tmpfile,
    error::{self, Error},
};

/// Creates temporary files before `switch_root`.
///
/// Creates the file with the specified permissions and parent directories.
pub fn pre(config: &Tmpfile) -> Result<(), Error> {
    let path = Path::new(&config.path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|_| error::CreateDirectorySnafu { path: parent.to_path_buf() })?;
    }

    let mode = u32::from_str_radix(&config.mode, 8).unwrap_or(0o644);

    // Create the file (empty)
    let _file = std::fs::File::create(path)
        .with_context(|_| error::CreateDirectorySnafu { path: config.path.clone() })?;

    // Set file permissions
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .with_context(|_| error::CreateDirectorySnafu { path: config.path.clone() })?;

    tracing::debug!(
        "Pre-switch: created tmpfile {} with mode {}",
        config.path.display(),
        config.mode
    );
    Ok(())
}

/// Creates temporary files after `switch_root`.
///
/// Currently a placeholder - post-switch tmpfile creation is not yet
/// implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &Tmpfile) -> Result<(), Error> {
    tracing::debug!("Post-switch: tmpfiles (not implemented)");
    Ok(())
}
