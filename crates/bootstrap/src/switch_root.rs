use snafu::ResultExt;

use crate::{ShellConfig, config::Config, error, phase, shutdown::shutdown};

/// Performs `switch_root` using `pivot_root` without exec.
///
/// This moves the special filesystems, performs `pivot_root` to switch to the
/// new root filesystem, and cleans up the old root.
///
/// # Errors
///
/// Returns an error if mount operations or `pivot_root` fails.
pub fn only(config: &Config) -> Result<(), error::Error> {
    phase::mount_move_special(&[])?;

    nix::mount::mount(
        None::<&str>,
        "/",
        None::<&str>,
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .with_context(|_| error::MountSnafu { operation: "namespace isolation in switch_root" })?;

    let old_root_dir = config.switch_root.old_root_dir.as_deref().unwrap_or("/oldroot");
    let cleanup_old_root = config.switch_root.cleanup_old_root;

    match config.switch_root.method {
        crate::config::SwitchRootMethod::PivotRoot => {
            let _ =
                nix::unistd::mkdir(old_root_dir, nix::sys::stat::Mode::from_bits_truncate(0o755));

            nix::unistd::pivot_root(".", old_root_dir).context(error::SwitchRootSnafu)?;

            nix::unistd::chdir("/").context(error::SwitchRootSnafu)?;

            nix::mount::umount2(old_root_dir, nix::mount::MntFlags::MNT_DETACH)
                .context(error::SwitchRootSnafu)?;

            if cleanup_old_root {
                drop(std::fs::remove_dir(old_root_dir));
            }
        }
        crate::config::SwitchRootMethod::Chroot => {
            nix::unistd::chdir("/newroot").context(error::SwitchRootSnafu)?;
            nix::unistd::chroot(".").context(error::SwitchRootSnafu)?;
            nix::unistd::chdir("/").context(error::SwitchRootSnafu)?;
        }
    }

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
) -> Result<(), error::Error> {
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
    ShellConfig { program, arguments: args, .. }: &ShellConfig,
) -> Result<(), error::Error> {
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

// (removed deprecated functions)
