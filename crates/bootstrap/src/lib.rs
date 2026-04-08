mod cmdline;
mod config;
mod error;
mod mount;
mod phase;
mod script;
mod shutdown;
mod switch_root;

pub use self::{
    cmdline::get_config_path,
    config::{
        Apparmor, BootScriptConfig, Clock, Config, Handoff, HandoffMode, HookSpec, InterfaceConfig,
        ModulesConfig, MountFailurePolicy, MountSource, MountSpec, NetworkConfig, NetworkMode,
        OnFailureConfig, OnFailurePolicy, OverlaySpec, PostSwitchPhase, PreSwitchPhase, RootConfig,
        Security, Selinux, ShellConfig, Shutdown, SwitchRootPhase, Symlink, SymlinkSpec, Sysctl,
        Tmpfile, VirtiofsMount,
    },
    error::Error,
    shutdown::shutdown,
};

/// Executes the bootstrap flow for supervise mode.
///
/// The initialization follows a three-tier phased architecture:
///
/// 1. Mounts virtual filesystems (`mount_virtual_filesystems`)
/// 2. Executes pre-switch phase functions (`phase::*_pre`)
///    - Clock configuration
///    - Sysctl settings
///    - Temporary files creation
///    - Symlinks creation
///    - Environment variables
///    - Kernel modules loading
///    - Network configuration
///    - Mounts (including root filesystem)
///    - Hook commands
/// 3. Switches root (`switch_root::only`)
/// 4. Executes post-switch phase functions (`phase::*_post`)
///    - Hooks
///    - Symlinks
///    - Environment variables
///    - Temporary files
///    - Sysctl settings
///    - Mounts
///    - Network configuration
///    - Kernel modules
///    - Security (SELinux/AppArmor)
///    - Clock configuration
/// 5. Executes the boot script if configured
/// 6. Hands off to the supervise orchestrator (`switch_root::exec_supervise`)
///
/// This function never returns on success — after `switch_root` it execs
/// into the supervise orchestrator.
///
/// # Errors
///
/// Returns an error if any boot stage fails.
pub fn execute(config: &Config) -> Result<(), Error> {
    let pid = nix::unistd::getpid();
    if pid.as_raw() == 1 {
        tracing::info!("Bootstrap started as PID 1");
    } else {
        tracing::warn!("Bootstrap should be PID 1, current PID: {pid}");
    }

    tracing::info!("Mounting virtual filesystems");
    mount::mount_virtual_filesystems()?;

    tracing::info!("Executing pre-switch phase functions");
    if let Some(clock) = &config.pre_switch.clock {
        phase::clock_pre(clock)?;
    }

    phase::sysctl_pre(&config.pre_switch.sysctl)?;

    if let Some(tmpfiles) = &config.pre_switch.tmpfiles {
        phase::tmpfiles_pre(tmpfiles)?;
    }
    phase::symlinks_pre(&config.pre_switch.symlinks)?;
    phase::environment_pre(&config.pre_switch.environment);
    if let Some(modules) = &config.pre_switch.modules {
        phase::modules_pre(modules);
    }
    if let Some(network) = &config.pre_switch.network {
        phase::network_pre(network)?;
    }
    phase::mounts_pre(&config.pre_switch.mounts)?;
    phase::hooks_pre(&config.pre_switch.hooks)?;

    tracing::info!("Switching root");
    switch_root::only(config)?;

    tracing::info!("Executing post-switch phase functions");
    phase::hooks_post(&config.post_switch.hooks)?;
    phase::symlinks_post(&config.post_switch.symlinks)?;
    phase::environment_post(&config.post_switch.environment);
    if let Some(tmpfiles) = &config.post_switch.tmpfiles {
        phase::tmpfiles_post(tmpfiles)?;
    }

    phase::sysctl_post(&config.post_switch.sysctl)?;
    phase::mounts_post(&config.post_switch.mounts)?;

    if let Some(network) = &config.post_switch.network {
        phase::network_post(network)?;
    }
    if let Some(modules) = &config.post_switch.modules {
        phase::modules_post(modules);
    }
    if let Some(security) = &config.post_switch.security {
        phase::security_post(security)?;
    }
    if let Some(clock) = &config.post_switch.clock {
        phase::clock_post(clock)?;
    }

    // Execute boot script if configured in handoff
    let handoff = &config.post_switch.handoff;
    if let Some(boot_script) = &handoff.boot_script {
        tracing::info!("Executing boot script");
        script::execute_boot_script(boot_script)?;
    }

    match &handoff.mode {
        HandoffMode::Supervise(orchestrator) => {
            tracing::info!("Handing off to supervise orchestrator");
            switch_root::exec_supervise(orchestrator.clone())?;
        }
        HandoffMode::Shell(shell) => {
            tracing::info!("Spawning interactive shell");
            switch_root::exec_shell(&config.console, shell)?;
        }
    }

    Ok(())
}
