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
pub fn switch_root_shell(
    console_device: &str,
    ShellConfig { program, args }: &ShellConfig,
) -> Result<(), error::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot").context(error::SwitchRootSnafu)?;
    unistd::chroot(".").context(error::SwitchRootSnafu)?;
    unistd::chdir("/").context(error::SwitchRootSnafu)?;

    // Use execute_interactive_with_session with create_session=false so the
    // shell inherits PID 1's session and controlling terminal. This also
    // provides proper signal handling and zombie reaping via signalfd.
    let exit_code = {
        let args = args.iter().map(String::as_str).collect::<Vec<&str>>();
        ocelot_entry::execute_interactive_with_session(program, &args, console_device, false, None)
            .context(error::ExecuteShellSnafu)?
    };

    tracing::info!("Shell exited with code: {exit_code}");

    if let Err(err) = shutdown() {
        tracing::error!("Failed to shutdown: {err}");
    }

    Ok(())
}
