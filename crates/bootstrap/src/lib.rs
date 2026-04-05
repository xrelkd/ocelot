mod cmdline;
mod config;
mod error;
mod modules;
mod mount;
mod script;
mod shutdown;
mod switch_root;
mod symlinks;
mod virtiofs;

use std::path::PathBuf;

use nix::unistd;
use snafu::ResultExt;

pub use self::{
    cmdline::get_config_path,
    config::{
        BootScriptConfig, Config, ModuleConfig, ModulesConfig, OnFailureConfig, OnFailurePolicy,
        RootConfig, ShellConfig, SymlinkSpec, VirtiofsMount,
    },
    error::Error,
    shutdown::shutdown,
};

/// Executes the bootstrap flow.
///
/// Initializes the environment for a QEMU VM boot:
/// 1. Mounts virtual filesystems
/// 2. Loads kernel modules
/// 3. Mounts root filesystem
/// 4. Sets up overlay if configured
/// 5. Mounts extra virtiofs shares
/// 6. Creates symlinks
/// 7. Sets environment variables and working directory
/// 8. Switches root and executes boot script
/// 9. Hands off to supervise
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

    tracing::info!("Mounting virtual filesystems");
    mount::mount_virtual_filesystems()?;

    tracing::info!("Loading kernel modules");
    if let Some(modules_config) = &config.modules {
        modules::load_modules(modules_config);
    }

    tracing::info!("Mounting root filesystem");
    mount::mount_root(&config.root)?;

    if config.root.overlay() {
        tracing::info!("Setting up overlay filesystem");
        mount::mount_overlay(&config.root)?;
    }

    if !config.extra_virtiofs_mounts.is_empty() {
        tracing::info!("Checking virtiofs support");
        virtiofs::check_virtiofs_support()?;

        tracing::info!("Mounting extra virtiofs shares");
        virtiofs::mount_extra_virtiofs(&config.extra_virtiofs_mounts);
    }

    if !config.symlinks.is_empty() {
        tracing::info!("Creating symlinks");
        symlinks::create_symlinks(&config.symlinks)?;
    }

    for (key, value) in &config.environment_variables {
        tracing::debug!("Setting environment variable: {key}={value}");
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
    }

    if let Some(dir) = &config.working_directory {
        tracing::info!("Changing working directory to: {dir}");
        std::env::set_current_dir(dir).with_context(|_| {
            error::FailedToChangeWorkingDirectorySnafu { path: PathBuf::from(dir) }
        })?;
    }

    tracing::info!("Switching root and handing off to supervise");
    switch_root::switch_root(orchestrator)?;

    if let Some(boot_script) = &config.boot_script {
        tracing::info!("Executing boot script");
        script::execute_boot_script(boot_script)?;
    }

    Ok(())
}

/// Executes the bootstrap flow in shell mode for debugging.
///
/// Initializes the environment for a QEMU VM boot:
/// 1. Mounts virtual filesystems
/// 2. Loads kernel modules
/// 3. Mounts root filesystem
/// 4. Sets up overlay if configured
/// 5. Mounts extra virtiofs shares
/// 6. Creates symlinks
/// 7. Sets environment variables and working directory
/// 8. Switches root and executes boot script
/// 9. Spawns an interactive shell
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

    tracing::info!("Mounting virtual filesystems");
    mount::mount_virtual_filesystems()?;

    tracing::info!("Loading kernel modules");
    if let Some(modules_config) = &config.modules {
        modules::load_modules(modules_config);
    }

    tracing::info!("Mounting root filesystem");
    mount::mount_root(&config.root)?;

    if config.root.overlay() {
        tracing::info!("Setting up overlay filesystem");
        mount::mount_overlay(&config.root)?;
    }

    if !config.extra_virtiofs_mounts.is_empty() {
        tracing::info!("Checking virtiofs support");
        virtiofs::check_virtiofs_support()?;

        tracing::info!("Mounting extra virtiofs shares");
        virtiofs::mount_extra_virtiofs(&config.extra_virtiofs_mounts);
    }

    if !config.symlinks.is_empty() {
        tracing::info!("Creating symlinks");
        symlinks::create_symlinks(&config.symlinks)?;
    }

    for (key, value) in &config.environment_variables {
        tracing::debug!("Setting environment variable: {key}={value}");
        #[expect(unsafe_code, reason = "Safe in PID 1 single-threaded context")]
        unsafe {
            std::env::set_var(key, value);
        }
    }

    if let Some(dir) = &config.working_directory {
        tracing::info!("Changing working directory to: {dir}");
        std::env::set_current_dir(dir).with_context(|_| {
            error::FailedToChangeWorkingDirectorySnafu { path: PathBuf::from(dir) }
        })?;
    }

    tracing::info!("Switching root and spawning shell: {}", shell_config.program);
    switch_root::switch_root_shell(&config.console, shell_config)?;

    if let Some(boot_script) = &config.boot_script {
        tracing::info!("Executing boot script");
        script::execute_boot_script(boot_script)?;
    }

    Ok(())
}
