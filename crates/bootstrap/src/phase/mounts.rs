/// Mount phase functions.
///
/// These functions mount filesystems during the bootstrap process.
use std::path::Path;

use crate::{config::MountSpec, error::Error, mount};

/// Mounts filesystems before `switch_root`.
///
/// Skips the root filesystem (already mounted by initramfs).
pub fn pre(specs: &[MountSpec]) -> Result<(), Error> {
    let root = Path::new("/");
    for spec in specs {
        if spec.target == root {
            continue;
        }
        let target = mount::mount(spec)?;
        tracing::debug!(
            "Pre-switch mounted {} at {} with flags: {:?}",
            spec.fstype,
            target.display(),
            spec.flags
        );
    }
    Ok(())
}

/// Mounts filesystems after `switch_root`.
///
/// Skips the root filesystem (already mounted).
pub fn post(specs: &[MountSpec]) -> Result<(), Error> {
    let root = Path::new("/");
    for spec in specs {
        if spec.target == root {
            continue;
        }
        let target = mount::mount(spec)?;

        tracing::debug!(
            "Post-switch mounted {} at {} with flags: {:?}",
            spec.fstype,
            target.display(),
            spec.flags
        );
    }
    Ok(())
}
