use nix::unistd;

use crate::{ShellConfig, mount, shutdown};

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

/// Performs `switch_root` and spawns an interactive shell.
///
/// After this call the process is running in the new root filesystem
/// and a shell is spawned with the console as controlling terminal.
///
/// Returns after the shell exits, then triggers system shutdown.
pub fn switch_root_shell(console: &str, shell_config: &ShellConfig) -> Result<(), nix::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot")?;
    unistd::chroot(".")?;
    unistd::chdir("/")?;

    let args: Vec<&str> = shell_config.args.iter().map(String::as_str).collect();
    let exit_code = ocelot_entry::execute_interactive(console, &shell_config.program, &args, None)
        .map_err(|_| nix::Error::EIO)?;

    tracing::info!("Shell exited with code: {exit_code}");

    if let Err(err) = shutdown() {
        tracing::error!("Failed to shutdown: {err}");
    }

    Ok(())
}
