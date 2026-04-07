use std::collections::HashMap;

use serde::Deserialize;

use crate::config::bootstrap::{
    core::{handoff::HandoffConfig, shutdown::ShutdownConfig},
    modules::ModulesConfig,
    mount::MountSpecConfig,
    network::NetworkConfig,
    security::SecurityConfig,
    supervise::HookSpecConfig,
    system::{ClockConfig, SymlinkConfig, SysctlConfig, TmpfileConfig},
};

/// `PreSwitchConfig`: Pre-switch configuration (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreSwitchConfig {
    #[serde(default)]
    pub modules: Option<ModulesConfig>,
    #[serde(default)]
    pub network: Option<NetworkConfig>,
    #[serde(default)]
    pub mounts: Vec<MountSpecConfig>,
    #[serde(default)]
    pub hooks: Vec<HookSpecConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub symlinks: Vec<SymlinkConfig>,
    #[serde(default)]
    pub sysctl: SysctlConfig,
    #[serde(default)]
    pub tmpfiles: Option<TmpfileConfig>,
    #[serde(default)]
    pub security: Option<SecurityConfig>,
    #[serde(default)]
    pub clock: Option<ClockConfig>,
}

/// `SwitchRootConfig`: Switch-root configuration (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SwitchRootConfig {
    #[serde(default)]
    pub method: SwitchRootMethod,
    #[serde(default)]
    pub old_root_dir: Option<String>,
    #[serde(default = "default_true")]
    pub cleanup_old_root: bool,
    #[serde(default = "default_true")]
    pub move_special: bool,
}

/// Switch-root method.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SwitchRootMethod {
    #[serde(rename = "pivotRoot")]
    #[default]
    PivotRoot,
    #[serde(rename = "chroot")]
    Chroot,
}

const fn default_true() -> bool { true }

/// `PostSwitchConfig`: Post-switch configuration (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PostSwitchConfig {
    #[serde(default)]
    pub modules: Option<ModulesConfig>,
    #[serde(default)]
    pub network: Option<NetworkConfig>,
    #[serde(default)]
    pub mounts: Vec<MountSpecConfig>,
    #[serde(default)]
    pub hooks: Vec<HookSpecConfig>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub symlinks: Vec<SymlinkConfig>,
    #[serde(default)]
    pub sysctl: SysctlConfig,
    #[serde(default)]
    pub tmpfiles: Option<TmpfileConfig>,
    #[serde(default)]
    pub security: Option<SecurityConfig>,
    #[serde(default)]
    pub clock: Option<ClockConfig>,
    #[serde(default)]
    pub handoff: HandoffConfig,
    #[serde(default)]
    pub shutdown: Option<ShutdownConfig>,
}

impl From<PreSwitchConfig> for ocelot_bootstrap::PreSwitchPhase {
    fn from(config: PreSwitchConfig) -> Self {
        Self {
            modules: config.modules.map(ocelot_bootstrap::ModulesConfig::from),
            network: config.network.map(ocelot_bootstrap::NetworkConfig::from),
            mounts: config.mounts.into_iter().map(ocelot_bootstrap::MountSpec::from).collect(),
            hooks: config.hooks.into_iter().map(ocelot_bootstrap::HookSpec::from).collect(),
            environment: config.environment,
            symlinks: config.symlinks.into_iter().map(ocelot_bootstrap::Symlink::from).collect(),
            sysctl: ocelot_bootstrap::Sysctl::from(config.sysctl),
            tmpfiles: config.tmpfiles.map(ocelot_bootstrap::Tmpfile::from),
            security: config.security.map(ocelot_bootstrap::Security::from),
            clock: config.clock.map(ocelot_bootstrap::Clock::from),
        }
    }
}

impl From<SwitchRootConfig> for ocelot_bootstrap::SwitchRootPhase {
    fn from(config: SwitchRootConfig) -> Self {
        Self {
            method: match config.method {
                SwitchRootMethod::PivotRoot => ocelot_bootstrap::SwitchRootMethod::PivotRoot,
                SwitchRootMethod::Chroot => ocelot_bootstrap::SwitchRootMethod::Chroot,
            },
            old_root_dir: config.old_root_dir,
            cleanup_old_root: config.cleanup_old_root,
            move_special: config.move_special,
        }
    }
}

impl From<PostSwitchConfig> for ocelot_bootstrap::PostSwitchPhase {
    fn from(config: PostSwitchConfig) -> Self {
        Self {
            modules: config.modules.map(ocelot_bootstrap::ModulesConfig::from),
            network: config.network.map(Into::into),
            mounts: config.mounts.into_iter().map(ocelot_bootstrap::MountSpec::from).collect(),
            hooks: config.hooks.into_iter().map(ocelot_bootstrap::HookSpec::from).collect(),
            environment: config.environment,
            symlinks: config.symlinks.into_iter().map(ocelot_bootstrap::Symlink::from).collect(),
            sysctl: ocelot_bootstrap::Sysctl::from(config.sysctl),
            tmpfiles: config.tmpfiles.map(ocelot_bootstrap::Tmpfile::from),
            security: config.security.map(ocelot_bootstrap::Security::from),
            clock: config.clock.map(ocelot_bootstrap::Clock::from),
            handoff: ocelot_bootstrap::Handoff::from(config.handoff),
            shutdown: config.shutdown.map(ocelot_bootstrap::Shutdown::from),
        }
    }
}
