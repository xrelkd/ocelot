use std::path::{Path, PathBuf};

use snafu::ResultExt;

use crate::{
    ShellConfig, SwitchRootPhase,
    error::{self, Error},
    mount,
    shutdown::shutdown,
};

/// Performs `switch_root` using `chroot` without exec.
///
/// This moves the special filesystems, performs `chroot` to switch to the
/// new root filesystem, and cleans up the old root.
///
/// # Errors
///
/// Returns an error if mount operations or `chroot` fails.
pub fn only(switch_root: &SwitchRootPhase) -> Result<(), Error> {
    let new_root = PathBuf::from("/new_root");

    ensure_dir(&new_root)?;
    {
        let mut root_file_system = switch_root.root_file_system.clone();
        root_file_system.target.clone_from(&new_root);
        let _unused = mount::mount(&root_file_system)?;
    }
    mount_move_special(&new_root, &[])?;

    clean_initramfs_mounts();

    if let Err(err) = recursive_remove_old_root(&new_root) {
        tracing::warn!("Failed to clean up some files in initramfs: {err}");
    }

    nix::unistd::chdir(&new_root)
        .with_context(|_| error::ChangeDirectorySnafu { path: new_root.clone() })?;

    nix::mount::mount(
        Some("."),
        &new_root,
        None::<&str>,
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_BIND,
        None::<&str>,
    )
    .with_context(|_| error::MountSnafu {
        link_source: new_root.clone(),
        target: PathBuf::new(),
    })?;

    nix::unistd::chroot(".")
        .with_context(|_| error::ChangeRootDirectorySnafu { path: PathBuf::from(".") })?;
    nix::unistd::chdir("/")
        .with_context(|_| error::ChangeDirectorySnafu { path: PathBuf::from("/") })?;

    Ok(())
}

/// Hands off to the supervise orchestrator.
///
/// This function never returns on success — it execs into the supervise
/// process.
///
/// # Errors
///
/// Returns an error if the supervise orchestrator fails to execute.
pub fn exec_supervise(
    orchestrator_config: ocelot_supervise::OrchestratorConfig,
) -> Result<(), Error> {
    let _exit_code =
        ocelot_supervise::execute(orchestrator_config).context(error::ExecuteSuperviseSnafu)?;
    Ok(())
}

/// Hands off to an interactive shell.
///
/// Spawns an interactive shell with the console as controlling terminal.
/// This function returns after the shell exits, then triggers system shutdown.
///
/// # Errors
///
/// Returns an error if the shell execution fails.
pub fn exec_shell(
    console_device: &str,
    ShellConfig { program, arguments, .. }: &ShellConfig,
) -> Result<(), Error> {
    let exit_code = {
        let args = arguments.iter().map(String::as_str).collect::<Vec<&str>>();
        ocelot_entry::execute_interactive_with_session(program, &args, console_device, true, None)
            .context(error::ExecuteShellSnafu)?
    };

    tracing::info!("Shell exited with code: {exit_code}");

    if let Err(err) = shutdown() {
        tracing::error!("Failed to shutdown: {err}");
    }

    Ok(())
}

/// Moves virtual filesystems from the old root to /newroot.
///
/// The following mounts are moved: /proc, /sys, /dev (with its subtree),
/// /run. The /dev subtree includes /dev/pts and /dev/shm which are moved
/// automatically when /dev is moved, so they don't need to be moved separately.
///
/// # Errors
///
/// Returns an error if any mount move operation fails.
fn mount_move_special(
    new_root_dir: impl AsRef<Path>,
    extra_targets: &[PathBuf],
) -> Result<(), Error> {
    let new_root = new_root_dir.as_ref().to_path_buf();

    move_mount("/proc", PathBuf::from_iter([&new_root, &PathBuf::from("proc")]))?;
    move_mount("/sys", PathBuf::from_iter([&new_root, &PathBuf::from("sys")]))?;
    move_mount("/dev", PathBuf::from_iter([&new_root, &PathBuf::from("dev")]))?;
    move_mount("/run", PathBuf::from_iter([&new_root, &PathBuf::from("run")]))?;

    for target in extra_targets {
        let newroot_target = PathBuf::from_iter([&new_root, target]);
        move_mount(target, newroot_target)?;
    }

    Ok(())
}

fn move_mount(source: impl AsRef<Path>, target: impl AsRef<Path>) -> Result<(), Error> {
    ensure_dir(&target)?;
    let source = source.as_ref();
    let target = target.as_ref();
    nix::mount::mount(
        Some(source),
        target,
        Option::<&str>::None,
        nix::mount::MsFlags::MS_MOVE,
        Option::<&str>::None,
    )
    .with_context(|_| error::MountSnafu {
        link_source: source.to_path_buf(),
        target: target.to_path_buf(),
    })?;
    Ok(())
}

fn ensure_dir(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();

    if path.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(path)
        .with_context(|_| error::CreateDirectorySnafu { path: path.to_path_buf() })
}

fn clean_initramfs_mounts() {
    // Read /proc/mounts to find everything currently mounted using procfs crate
    if let Ok(mounts) = procfs::mounts() {
        let mut mount_points = mounts
            .into_iter()
            .filter_map(
                |procfs::MountEntry { fs_file, .. }| {
                    if fs_file == "/" { None } else { Some(fs_file) }
                },
            )
            .collect::<Vec<_>>();

        // Sort by length descending (unmount deepest children first)
        mount_points.sort_by_key(|point| std::cmp::Reverse(point.len()));

        for mount_point in mount_points {
            // MNT_DETACH (Lazy) is safest in initramfs to avoid "device busy"
            // errors
            let _ = nix::mount::umount2(mount_point.as_str(), nix::mount::MntFlags::MNT_DETACH);
        }
    }
}

fn recursive_remove_old_root(new_root_path: impl AsRef<Path>) -> std::io::Result<()> {
    let new_root_path = new_root_path.as_ref();
    let root = Path::new("/");
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path == new_root_path {
            continue;
        }

        if path.is_dir() {
            drop(std::fs::remove_dir_all(&path));
        } else {
            drop(std::fs::remove_file(&path));
        }
    }
    Ok(())
}
