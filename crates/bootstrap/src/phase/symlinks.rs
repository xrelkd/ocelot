/// Symlink creation phase functions.
///
/// These functions create symbolic links during the bootstrap process.
use std::os::unix::fs::symlink;

use snafu::ResultExt;

use crate::{
    config::Symlink,
    error::{self, Error},
};

/// Creates symbolic links before `switch_root`.
///
/// Creates parent directories as needed and logs a warning if the target
/// does not exist.
pub fn pre(specs: &[Symlink]) -> Result<(), Error> {
    for spec in specs {
        let Symlink { source, target } = spec;

        if let Some(parent) = target.parent()
            && let Some(parent_str) = parent.to_str()
            && !parent_str.is_empty()
        {
            std::fs::create_dir_all(parent_str)
                .with_context(|_| error::CreateDirectorySnafu { path: target.clone() })?;
        }

        if !source.exists() {
            tracing::warn!(
                "Symlink target '{}' does not exist, creating symlink anyway",
                source.display()
            );
        }

        symlink(source, target).with_context(|_| error::CreateSymlinkSnafu {
            link_source: source.clone(),
            target: target.clone(),
        })?;

        tracing::info!("Pre-switch: created symlink {} -> {}", target.display(), source.display());
    }
    Ok(())
}

/// Creates symbolic links after `switch_root`.
///
/// Currently a placeholder - post-switch symlink creation is not yet
/// implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_specs: &[Symlink]) -> Result<(), Error> {
    tracing::debug!("Post-switch: symlinks (not implemented)");
    Ok(())
}
