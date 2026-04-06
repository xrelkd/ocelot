use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use nix::mount::MsFlags;
use snafu::ResultExt;

use crate::{
    config::{MountSpec, RootConfig},
    error,
    error::Error,
};

const DEVICE_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const DEVICE_WAIT_INTERVAL: Duration = Duration::from_millis(100);

pub fn pre(specs: &[MountSpec]) -> Result<(), Error> {
    for spec in specs {
        let target_path = if spec.target.as_os_str().is_empty() {
            Path::new("/newroot")
        } else {
            spec.target.as_path()
        };

        let flags = spec.flags;
        let options = spec.options.as_deref();

        // Build source path as a string, then convert to &Path
        let source_string = match spec.source {
            crate::config::MountSource::Device(ref d) => d.clone(),
            crate::config::MountSource::VirtiofsTag(ref t)
            | crate::config::MountSource::NinePTag(ref t) => t.clone(),
            crate::config::MountSource::Virtual => String::new(),
            crate::config::MountSource::Overlay(_) => "overlay".to_string(),
            crate::config::MountSource::Nfs { ref server, ref export } => {
                format!("{server}:{export}")
            }
        };
        let source_path = Path::new(&source_string);

        let fstype_path = Path::new(&spec.fstype);

        nix::mount::mount(Some(source_path), target_path, Some(fstype_path), flags, options)
            .with_context(|_| error::MountSnafu {
                operation: format!("mount {} to {}", spec.fstype, target_path.display()),
            })?;

        tracing::debug!(
            "Pre-switch: mounted {} at {} with flags: {:?}",
            spec.fstype,
            target_path.display(),
            flags
        );
    }
    Ok(())
}

pub fn post(specs: &[MountSpec]) -> Result<(), Error> {
    for spec in specs {
        let target_path =
            if spec.target.as_os_str().is_empty() { Path::new("/") } else { spec.target.as_path() };

        let flags = spec.flags;
        let options = spec.options.as_deref();

        let source_string = match spec.source {
            crate::config::MountSource::Device(ref d) => d.clone(),
            crate::config::MountSource::VirtiofsTag(ref t)
            | crate::config::MountSource::NinePTag(ref t) => t.clone(),
            crate::config::MountSource::Virtual => String::new(),
            crate::config::MountSource::Overlay(_) => "overlay".to_string(),
            crate::config::MountSource::Nfs { ref server, ref export } => {
                format!("{server}:{export}")
            }
        };
        let source_path = Path::new(&source_string);

        let fstype_path = Path::new(&spec.fstype);

        nix::mount::mount(Some(source_path), target_path, Some(fstype_path), flags, options)
            .with_context(|_| error::MountSnafu {
                operation: format!("mount {} to {}", spec.fstype, target_path.display()),
            })?;

        tracing::debug!(
            "Post-switch: mounted {} at {} with flags: {:?}",
            spec.fstype,
            target_path.display(),
            flags
        );
    }
    Ok(())
}

/// Mounts the standard virtual filesystems: /proc, /sys, /dev, /run.
///
/// # Errors
///
/// Returns an error if any mount operation fails.
pub fn mount_virtual_filesystems() -> Result<(), Error> {
    nix::mount::mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .with_context(|_| error::MountSnafu { operation: "mount namespace isolation" })?;

    mount_api_vfs("proc", "/proc", "proc")?;
    mount_api_vfs("sysfs", "/sys", "sysfs")?;
    mount_api_vfs("devtmpfs", "/dev", "devtmpfs")?;
    mount_api_vfs("tmpfs", "/run", "tmpfs")?;
    mount_api_vfs("devpts", "/dev/pts", "devpts")?;
    mount_api_vfs("tmpfs", "/dev/shm", "tmpfs")?;
    mount_api_vfs("tmpfs", "/tmp", "tmpfs")?;

    fs::create_dir_all("/run/lock")
        .with_context(|_| error::CreateDirectorySnafu { path: PathBuf::from("/run/lock") })?;

    Ok(())
}

/// Mounts the root filesystem to `/newroot` based on configuration.
///
/// # Errors
///
/// Returns an error if the device is not found or mount operation fails.
#[expect(dead_code, reason = "Deprecated, use phase::mounts_pre instead")]
pub fn mount_root(config: &RootConfig) -> Result<(), Error> {
    let source = config.source();
    let fstype = config.fstype();

    if matches!(config, RootConfig::Block { .. }) {
        wait_for_device(source)?;
    }

    ensure_dir("/newroot")?;

    let data = config.mount_options();
    nix::mount::mount(Some(source), "/newroot", Some(fstype), MsFlags::empty(), data)
        .with_context(|_| error::MountSnafu { operation: "root filesystem at /newroot" })?;

    tracing::info!("Mounted {fstype} {source} at /newroot");
    Ok(())
}

/// Sets up overlayfs on top of the mounted root filesystem.
///
/// Uses isolated directories per mount source under `/run/overlayfs/{source}/`.
///
/// # Errors
///
/// Returns an error if directory creation or mount operation fails.
#[expect(dead_code, reason = "Deprecated, use phase::mounts_pre instead")]
pub fn mount_overlay(config: &RootConfig) -> Result<(), Error> {
    let source = config.source();
    let base = overlay_base(source);
    let upper = format!("{base}/upper");
    let work = format!("{base}/work");

    ensure_dir(&upper)?;
    ensure_dir(&work)?;

    let opts = format!("lowerdir=/newroot,upperdir={upper},workdir={work}");
    nix::mount::mount(
        Some("overlay"),
        "/newroot",
        Some("overlay"),
        MsFlags::empty(),
        Some(opts.as_str()),
    )
    .with_context(|_| error::MountSnafu { operation: "overlayfs on /newroot" })?;

    tracing::info!("Mounted overlayfs on /newroot (source: {source})");
    Ok(())
}

/// Returns the base directory for overlay files for a given mount source.
///
/// Sanitizes the source name to prevent path traversal.
fn overlay_base(source: &str) -> String {
    let safe_name = source.replace('/', "_");
    format!("/run/overlayfs/{safe_name}")
}

/// Moves virtual filesystems from the old root to /newroot.
///
/// # Errors
///
/// Returns an error if any mount move operation fails.
pub fn mount_move_special(extra_targets: &[PathBuf]) -> Result<(), Error> {
    move_mount("/proc", "/newroot/proc")?;
    move_mount("/sys", "/newroot/sys")?;
    move_mount("/dev", "/newroot/dev")?;
    move_mount("/run", "/newroot/run")?;
    move_mount("/dev/pts", "/newroot/dev/pts")?;
    move_mount("/dev/shm", "/newroot/dev/shm")?;

    for target in extra_targets {
        let newroot_target = format!("/newroot{}", target.display());
        move_mount(&target.to_string_lossy(), &newroot_target)?;
    }

    Ok(())
}

fn mount_api_vfs(source: &str, target: &str, fstype: &str) -> Result<(), Error> {
    ensure_dir(target)?;
    nix::mount::mount(Some(source), target, Some(fstype), MsFlags::empty(), Option::<&str>::None)
        .with_context(|_| error::MountSnafu { operation: format!("{fstype} at {target}") })?;
    tracing::info!("Mounted {fstype} at {target}");
    Ok(())
}

fn move_mount(source: &str, target: &str) -> Result<(), Error> {
    ensure_dir(target)?;
    nix::mount::mount(
        Some(Path::new(source)),
        target,
        Option::<&str>::None,
        MsFlags::MS_MOVE,
        Option::<&str>::None,
    )
    .with_context(|_| error::MountSnafu { operation: format!("{source} to {target}") })?;
    Ok(())
}

fn ensure_dir(path: &str) -> Result<(), Error> {
    fs::create_dir_all(path)
        .with_context(|_| error::CreateDirectorySnafu { path: PathBuf::from(path) })
}

fn wait_for_device(device: &str) -> Result<(), Error> {
    let path = Path::new(device);
    let start = std::time::Instant::now();

    while start.elapsed() < DEVICE_WAIT_TIMEOUT {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(DEVICE_WAIT_INTERVAL);
    }

    Err(Error::Mount { operation: format!("wait for device {device}"), source: nix::Error::ENODEV })
}

#[expect(dead_code, reason = "reserved for future cleanup support")]
pub fn umount_old_root() {
    use nix::mount::umount;
    let _ = umount("/oldroot");
    drop(fs::remove_dir("/oldroot"));
}

#[cfg(test)]
mod tests {
    use super::overlay_base;

    #[test]
    fn test_overlay_base_simple_tag() {
        assert_eq!(overlay_base("myshare"), "/run/overlayfs/myshare");
    }

    #[test]
    fn test_overlay_base_device_path() {
        // Slashes in device paths are replaced with underscores
        assert_eq!(overlay_base("/dev/vda2"), "/run/overlayfs/_dev_vda2");
    }

    #[test]
    fn test_overlay_base_complex_path() {
        assert_eq!(overlay_base("virtiofs/my-tag"), "/run/overlayfs/virtiofs_my-tag");
    }
}
