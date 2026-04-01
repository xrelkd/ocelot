use nix::unistd;

use crate::mount;

/// Performs `switch_root`: move mounts, chroot, and hand off to supervise.
///
/// After this call the process is running in the new root filesystem
/// and the supervise orchestrator takes over.
pub fn switch_root(
    orchestrator_config: ocelot_supervise::OrchestratorConfig,
) -> Result<(), nix::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot")?;
    unistd::chroot(".")?;
    unistd::chdir("/")?;

    let _ = ocelot_supervise::execute(orchestrator_config).map_err(|_| nix::Error::EIO)?;
    Ok(())
}
