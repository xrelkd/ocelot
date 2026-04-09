/// Tmpfile creation phase functions.
///
/// These functions create temporary files during the bootstrap process.
use std::{os::unix::fs::PermissionsExt, path::Path};

use snafu::ResultExt;

use crate::{config::Tmpfile, error, error::Error};

/// Creates temporary files before `switch_root`.
///
/// Creates the file with the specified permissions and parent directories.
pub fn pre(config: &Tmpfile) -> Result<(), Error> {
    create_file(config)?;
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
pub fn post(config: &Tmpfile) -> Result<(), Error> {
    create_file(config)?;
    tracing::debug!(
        "Post-switch: created tmpfile {} with mode {}",
        config.path.display(),
        config.mode
    );
    Ok(())
}

fn create_file(config: &Tmpfile) -> Result<(), Error> {
    let path = Path::new(&config.path);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|_| error::CreateDirectorySnafu { path: parent.to_path_buf() })?;
    }

    let mode = u32::from_str_radix(&config.mode, 8).unwrap_or(0o644);

    // Create the file (empty)
    let _file = std::fs::File::create(path)
        .with_context(|_| error::CreateFileSnafu { path: config.path.clone() })?;

    // Set file permissions
    let permissions = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, permissions.clone()).with_context(|_| {
        error::SetPermissionsSnafu { permissions, target: config.path.clone() }
    })?;

    Ok(())
}
