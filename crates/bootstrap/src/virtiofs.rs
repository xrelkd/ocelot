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
        let VirtiofsMount { tag, with_overlay, .. } = mount_spec;
        if let Err(source) = mount_virtiofs_share(mount_spec) {
            tracing::warn!("Failed to mount virtiofs share '{tag}': {source}");
            continue;
        }

        if *with_overlay && let Err(source) = mount_overlay_for_share(mount_spec) {
            tracing::warn!("Failed to set up overlay for virtiofs share '{tag}': {source}");
        }
    }
}

/// Mounts a single virtiofs share.
pub fn mount_virtiofs_share(
    VirtiofsMount { tag, path, options, .. }: &VirtiofsMount,
) -> Result<(), Error> {
    ensure_dir_all(path)?;

    let data = options.as_deref();
    mount(Some(tag.as_str()), path.as_str(), Some("virtiofs"), MsFlags::empty(), data)
        .with_context(|_| error::MountSnafu { operation: format!("virtiofs '{tag}' at {path}") })?;

    tracing::info!("Mounted virtiofs '{tag}' at {path}");
    Ok(())
}

/// Sets up overlayfs on top of a mounted virtiofs share.
///
/// Uses isolated directories under `/run/overlayfs/{tag}/`.
pub fn mount_overlay_for_share(
    VirtiofsMount { tag, path, .. }: &VirtiofsMount,
) -> Result<(), Error> {
    let base = overlay_share_base(tag);
    let upper = format!("{base}/upper");
    let work = format!("{base}/work");

    ensure_dir_all(&upper)?;
    ensure_dir_all(&work)?;

    let opts = format!("lowerdir={path},upperdir={upper},workdir={work}");
    mount(Some("overlay"), path.as_str(), Some("overlay"), MsFlags::empty(), Some(opts.as_str()))
        .with_context(|_| error::MountSnafu {
        operation: format!("overlayfs on {path} (tag: {tag})"),
    })?;

    tracing::info!("Mounted overlayfs on {path} (tag: {tag})");
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
