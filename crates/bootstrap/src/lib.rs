mod cmdline;
mod config;
mod console;
mod error;
mod modules;
mod mount;
mod shutdown;
mod switch_root;

use nix::unistd;
use snafu::ResultExt;

pub use self::{
    cmdline::get_config_path,
    config::{Config, ModuleConfig, OnFailureConfig, RootConfig, ShellConfig},
    error::Error,
    shutdown::shutdown,
};

/// Executes the bootstrap flow.
///
/// Initializes the environment for a QEMU VM boot:
/// 1. Sets up console
/// 2. Mounts virtual filesystems
/// 3. Loads kernel modules
/// 4. Mounts root filesystem
/// 5. Sets up overlay if configured
/// 6. Switches root and hands off to supervise
///
/// This function never returns on success — after `switch_root` it execs
/// into the supervise orchestrator.
///
/// # Errors
///
/// Returns an error if any boot stage fails.
pub fn execute_supervise(
    config: &Config,
    orchestrator: ocelot_supervise::OrchestratorConfig,
) -> Result<(), Error> {
    let pid = unistd::getpid();
    if pid.as_raw() == 1 {
        tracing::info!("Bootstrap started as PID 1");
    } else {
        tracing::warn!("Bootstrap should be PID 1, current PID: {pid}");
    }

    tracing::info!("Setting up console");
    console::setup(&config.console).context(error::ConsoleSetupSnafu)?;

    tracing::info!("Mounting virtual filesystems");
    mount::mount_virtual_filesystems()
        .context(error::MountSnafu { operation: "virtual filesystems" })?;

    tracing::info!("Loading kernel modules");
    if let Some(modules_config) = &config.modules {
        modules::load_modules(modules_config);
    }

    tracing::info!("Mounting root filesystem");
    mount::mount_root(&config.root).context(error::MountSnafu { operation: "root filesystem" })?;

    if config.root.overlay() {
        tracing::info!("Setting up overlay filesystem");
        mount::mount_overlay(&config.root)
            .context(error::MountSnafu { operation: "overlay filesystem" })?;
    }

    for (key, value) in &config.environment_variables {
        tracing::debug!("Setting environment variable: {}={}", key, value);
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
    }

    if let Some(dir) = &config.working_directory {
        tracing::info!("Changing working directory to: {}", dir);
        std::env::set_current_dir(dir)
            .context(error::FailedToChangeWorkingDirectorySnafu { path: dir })?;
    }

    tracing::info!("Switching root and handing off to supervise");
    switch_root::switch_root(orchestrator).context(error::SwitchRootSnafu)?;

    Ok(())
}

/// Executes the bootstrap flow in shell mode for debugging.
///
/// Initializes the environment for a QEMU VM boot:
/// 1. Sets up console
/// 2. Mounts virtual filesystems
/// 3. Loads kernel modules
/// 4. Mounts root filesystem
/// 5. Sets up overlay if configured
/// 6. Switches root and spawns an interactive shell
///
/// This function never returns on success — after `switch_root` it execs
/// into the specified shell.
///
/// # Errors
///
/// Returns an error if any boot stage fails.
pub fn execute_shell(config: &Config, shell_config: &ShellConfig) -> Result<(), Error> {
    let pid = unistd::getpid();
    if pid.as_raw() == 1 {
        tracing::info!("Bootstrap (shell mode) started as PID 1");
    } else {
        tracing::warn!("Bootstrap should be PID 1, current PID: {pid}");
    }

    tracing::info!("Setting up console");
    console::setup(&config.console).context(error::ConsoleSetupSnafu)?;

    tracing::info!("Mounting virtual filesystems");
    mount::mount_virtual_filesystems()
        .context(error::MountSnafu { operation: "virtual filesystems" })?;

    tracing::info!("Loading kernel modules");
    if let Some(modules_config) = &config.modules {
        modules::load_modules(modules_config);
    }

    tracing::info!("Mounting root filesystem");
    mount::mount_root(&config.root).context(error::MountSnafu { operation: "root filesystem" })?;

    if config.root.overlay() {
        tracing::info!("Setting up overlay filesystem");
        mount::mount_overlay(&config.root)
            .context(error::MountSnafu { operation: "overlay filesystem" })?;
    }

    for (key, value) in &config.environment_variables {
        tracing::debug!("Setting environment variable: {}={}", key, value);
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
    }

    if let Some(dir) = &config.working_directory {
        tracing::info!("Changing working directory to: {}", dir);
        std::env::set_current_dir(dir)
            .context(error::FailedToChangeWorkingDirectorySnafu { path: dir })?;
    }

    tracing::info!("Switching root and spawning shell: {}", shell_config.program);
    switch_root::switch_root_shell(&config.console, shell_config)
        .context(error::SwitchRootSnafu)?;

    Ok(())
}
