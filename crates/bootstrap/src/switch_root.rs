use nix::unistd;
use snafu::ResultExt;

use crate::{ShellConfig, error, mount, shutdown};

/// Performs `switch_root`: move mounts, chroot, and hand off to supervise.
///
/// After this call the process is running in the new root filesystem
/// and the supervise orchestrator takes over.
///
/// # Errors
///
/// Returns an error if mount operations fail, chroot fails,
/// or the supervise orchestrator fails to execute.
pub fn switch_root(
    orchestrator_config: ocelot_supervise::OrchestratorConfig,
) -> Result<(), error::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot").context(error::SwitchRootSnafu)?;
    unistd::chroot(".").context(error::SwitchRootSnafu)?;
    unistd::chdir("/").context(error::SwitchRootSnafu)?;

    let _exit_code =
        ocelot_supervise::execute(orchestrator_config).context(error::ExecuteSuperviseSnafu)?;
    Ok(())
}

/// Performs `switch_root` and spawns an interactive shell.
///
/// After this call the process is running in the new root filesystem
/// and a shell is spawned with the console as controlling terminal.
///
/// Returns after the shell exits, then triggers system shutdown.
///
/// # Errors
///
/// Returns an error if mount operations fail, chroot fails,
/// or the shell execution fails.
pub fn switch_root_shell(console: &str, shell_config: &ShellConfig) -> Result<(), error::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot").context(error::SwitchRootSnafu)?;
    unistd::chroot(".").context(error::SwitchRootSnafu)?;
    unistd::chdir("/").context(error::SwitchRootSnafu)?;

    let args = shell_config.args.iter().map(String::as_str).collect::<Vec<&str>>();
    let exit_code = ocelot_entry::execute_interactive(console, &shell_config.program, &args, None)
        .context(error::ExecuteShellSnafu)?;

    tracing::info!("Shell exited with code: {exit_code}");

    if let Err(err) = shutdown() {
        tracing::error!("Failed to shutdown: {err}");
    }

    Ok(())
}
