use std::{
    fs::{self},
    path::Path,
    thread,
    time::Duration,
};

use nix::mount::{MsFlags, mount};

use crate::config::RootConfig;

const DEVICE_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const DEVICE_WAIT_INTERVAL: Duration = Duration::from_millis(100);

/// Mounts the standard virtual filesystems: /proc, /sys, /dev, /run.
pub fn mount_virtual_filesystems() -> Result<(), nix::Error> {
    mount_api_vfs("proc", "/proc", "proc")?;
    mount_api_vfs("sysfs", "/sys", "sysfs")?;
    mount_api_vfs("devtmpfs", "/dev", "devtmpfs")?;
    mount_api_vfs("tmpfs", "/run", "tmpfs")?;
    Ok(())
}

/// Mounts the root filesystem to `/newroot` based on configuration.
pub fn mount_root(config: &RootConfig) -> Result<(), nix::Error> {
    let source = config.source();
    let fstype = config.fstype();

    if matches!(config, RootConfig::Block { .. }) {
        wait_for_device(source)?;
    }

    ensure_dir("/newroot")?;

    let data = config.mount_options();
    mount(Some(source), "/newroot", Some(fstype), MsFlags::empty(), data)?;

    tracing::info!("Mounted {fstype} {source} at /newroot");
    Ok(())
}

/// Sets up overlayfs on top of the mounted root filesystem.
pub fn mount_overlay(_config: &RootConfig) -> Result<(), nix::Error> {
    let upper = "/run/overlay/upper";
    let work = "/run/overlay/work";

    ensure_dir(upper)?;
    ensure_dir(work)?;

    mount(
        Some("overlay"),
        "/newroot",
        Some("overlay"),
        MsFlags::empty(),
        Some("lowerdir=/newroot,upperdir=/run/overlay/upper,workdir=/run/overlay/work"),
    )?;

    tracing::info!("Mounted overlayfs on /newroot");
    Ok(())
}

/// Moves virtual filesystems from the old root to /newroot.
pub fn mount_move_special() -> Result<(), nix::Error> {
    move_mount("/proc", "/newroot/proc")?;
    move_mount("/sys", "/newroot/sys")?;
    move_mount("/dev", "/newroot/dev")?;
    move_mount("/run", "/newroot/run")?;
    Ok(())
}

fn mount_api_vfs(source: &str, target: &str, fstype: &str) -> Result<(), nix::Error> {
    ensure_dir(target)?;
    mount(Some(source), target, Some(fstype), MsFlags::empty(), Option::<&str>::None)?;
    tracing::info!("Mounted {fstype} at {target}");
    Ok(())
}

fn move_mount(source: &str, target: &str) -> Result<(), nix::Error> {
    ensure_dir(target)?;
    mount(
        Some(Path::new(source)),
        target,
        Option::<&str>::None,
        MsFlags::MS_MOVE,
        Option::<&str>::None,
    )?;
    Ok(())
}

fn ensure_dir(path: &str) -> Result<(), nix::Error> {
    fs::create_dir_all(path).map_err(|_| nix::Error::EIO)
}

fn wait_for_device(device: &str) -> Result<(), nix::Error> {
    let path = Path::new(device);
    let start = std::time::Instant::now();

    while start.elapsed() < DEVICE_WAIT_TIMEOUT {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(DEVICE_WAIT_INTERVAL);
    }

    Err(nix::Error::ENODEV)
}

#[expect(dead_code, reason = "reserved for future cleanup support")]
pub fn umount_old_root() {
    use nix::mount::umount;
    let _ = umount("/oldroot");
    drop(fs::remove_dir("/oldroot"));
}
