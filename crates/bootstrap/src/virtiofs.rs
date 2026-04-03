use std::fs;

use nix::mount::{MsFlags, mount};
use snafu::ResultExt;

use crate::{
    config::VirtiofsMount,
    error::{self, Error},
};

/// Checks if the kernel supports virtiofs by reading `/proc/filesystems`.
///
/// # Errors
///
/// Returns an error if virtiofs is not supported or if `/proc/filesystems`
/// cannot be read.
pub fn check_virtiofs_support() -> Result<(), Error> {
    let contents =
        fs::read_to_string("/proc/filesystems").with_context(|_| error::ReadFilesystemsSnafu)?;

    if contents.lines().any(|line: &str| line.contains("virtiofs")) {
        Ok(())
    } else {
        error::VirtiofsNotSupportedSnafu { message: "Kernel does not support virtiofs filesystem" }
            .fail()
    }
}

/// Mounts extra virtiofs shares as configured.
///
/// Iterates over each mount spec, mounting the virtiofs share and optionally
/// setting up overlayfs.
pub fn mount_extra_virtiofs(mounts: &[VirtiofsMount]) {
    for mount_spec in mounts {
        if let Err(source) = mount_virtiofs_share(mount_spec) {
            tracing::warn!("Failed to mount virtiofs share '{}': {source}", mount_spec.tag);
            continue;
        }

        if mount_spec.with_overlay
            && let Err(source) = mount_overlay_for_share(mount_spec)
        {
            tracing::warn!(
                "Failed to set up overlay for virtiofs share '{}': {source}",
                mount_spec.tag
            );
        }
    }
}

/// Mounts a single virtiofs share.
pub fn mount_virtiofs_share(spec: &VirtiofsMount) -> Result<(), Error> {
    ensure_dir_all(&spec.path)?;

    let data = spec.options.as_deref();
    mount(Some(spec.tag.as_str()), spec.path.as_str(), Some("virtiofs"), MsFlags::empty(), data)
        .with_context(|_| error::MountSnafu {
            operation: format!("virtiofs '{}' at {}", spec.tag, spec.path),
        })?;

    tracing::info!("Mounted virtiofs '{}' at {}", spec.tag, spec.path);
    Ok(())
}

/// Sets up overlayfs on top of a mounted virtiofs share.
///
/// Uses isolated directories under `/run/overlayfs/{tag}/`.
pub fn mount_overlay_for_share(spec: &VirtiofsMount) -> Result<(), Error> {
    let base = overlay_share_base(&spec.tag);
    let upper = format!("{base}/upper");
    let work = format!("{base}/work");

    ensure_dir_all(&upper)?;
    ensure_dir_all(&work)?;

    let opts = format!("lowerdir={},upperdir={upper},workdir={work}", spec.path);
    mount(
        Some("overlay"),
        spec.path.as_str(),
        Some("overlay"),
        MsFlags::empty(),
        Some(opts.as_str()),
    )
    .with_context(|_| error::MountSnafu {
        operation: format!("overlayfs on {} (tag: {})", spec.path, spec.tag),
    })?;

    tracing::info!("Mounted overlayfs on {} (tag: {})", spec.path, spec.tag);
    Ok(())
}

/// Returns the base directory for overlay files for a given virtiofs tag.
fn overlay_share_base(tag: &str) -> String {
    let safe_name = tag.replace('/', "_");
    format!("/run/overlayfs/{safe_name}")
}

/// Recursively creates a directory and all parent directories with mode 0755.
fn ensure_dir_all(path: &str) -> Result<(), Error> {
    fs::create_dir_all(path)
        .with_context(|_| error::CreateDirectorySnafu { path: path.to_string() })
}

#[cfg(test)]
mod tests {
    use super::overlay_share_base;

    #[test]
    fn test_overlay_share_base_simple_tag() {
        assert_eq!(overlay_share_base("myshare"), "/run/overlayfs/myshare");
    }

    #[test]
    fn test_overlay_share_base_with_slashes() {
        assert_eq!(overlay_share_base("virtiofs/my-tag"), "/run/overlayfs/virtiofs_my-tag");
    }

    #[test]
    fn test_overlay_share_base_complex_path() {
        assert_eq!(overlay_share_base("root/share/1"), "/run/overlayfs/root_share_1");
    }
}
