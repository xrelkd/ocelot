use std::path::{Path, PathBuf};

use snafu::ResultExt;

use crate::{
    config::{MountSource, MountSpec},
    error,
    error::Error,
};

/// Mounts a filesystem according to the given specification.
///
/// # Errors
///
/// Returns an error if the mount operation fails.
pub fn mount(spec: &MountSpec) -> Result<PathBuf, Error> {
    let target_path =
        if spec.target.as_os_str().is_empty() { Path::new("/") } else { &spec.target };

    // Ensure the target directory exists
    if !target_path.exists() {
        std::fs::create_dir_all(target_path)
            .with_context(|_| error::CreateDirectorySnafu { path: target_path.to_path_buf() })?;
    }

    let flags = spec.flags;
    let options = spec.options.as_deref();

    // Build source path as a string, then convert to &Path
    let source_string = match spec.source {
        MountSource::Device(ref d) => d.clone(),
        MountSource::VirtiofsTag(ref t) | MountSource::NinePTag(ref t) => t.clone(),
        MountSource::Virtual => String::new(),
        MountSource::Overlay(_) => "overlay".to_string(),
        MountSource::Nfs { ref server, ref export } => {
            format!("{server}:{export}")
        }
    };
    let source_path = Path::new(&source_string);

    let fstype_path = Path::new(&spec.fstype);

    nix::mount::mount(Some(source_path), target_path, Some(fstype_path), flags, options)
        .with_context(|_| error::MountSnafu {
            link_source: source_path.to_path_buf(),
            target: target_path.to_path_buf(),
        })?;

    Ok(target_path.to_path_buf())
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
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .with_context(|_| error::IsolateNamespaceSnafu)?;

    mount_api_vfs("proc", "/proc", "proc")?;
    mount_api_vfs("sysfs", "/sys", "sysfs")?;
    mount_api_vfs("devtmpfs", "/dev", "devtmpfs")?;
    mount_api_vfs("tmpfs", "/run", "tmpfs")?;
    mount_api_vfs("devpts", "/dev/pts", "devpts")?;
    mount_api_vfs("tmpfs", "/dev/shm", "tmpfs")?;
    mount_api_vfs("tmpfs", "/tmp", "tmpfs")?;

    std::fs::create_dir_all("/run/lock")
        .with_context(|_| error::CreateDirectorySnafu { path: PathBuf::from("/run/lock") })?;

    Ok(())
}

fn mount_api_vfs(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
    fstype: &str,
) -> Result<(), Error> {
    let source = source.as_ref();
    let target = target.as_ref();
    std::fs::create_dir_all(target)
        .with_context(|_| error::CreateDirectorySnafu { path: target.to_path_buf() })?;
    nix::mount::mount(
        Some(source),
        target,
        Some(fstype),
        nix::mount::MsFlags::empty(),
        Option::<&str>::None,
    )
    .with_context(|_| error::MountSnafu {
        link_source: source.to_path_buf(),
        target: target.to_path_buf(),
    })?;
    tracing::info!("Mounted {fstype} at {}", target.display());
    Ok(())
}
