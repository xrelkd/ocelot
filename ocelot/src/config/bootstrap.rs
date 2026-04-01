use std::{collections::HashMap, path::Path, time::Duration};

use serde::Deserialize;
use snafu::ResultExt;

use crate::config::{Error, ProcessConfig, SuperviseConfig, error};

/// Bootstrap configuration file structure.
///
/// This represents the YAML configuration for the bootstrap subcommand,
/// which combines bootstrap-specific options with supervise process
/// definitions.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootstrapConfig {
    /// Root filesystem configuration (virtiofs, block, or 9p).
    pub root: RootConfig,
    /// Optional kernel module loading configuration.
    #[serde(default)]
    pub modules: Option<ModulesConfig>,
    /// Console device to use (default: "console").
    #[serde(default = "default_console")]
    pub console: String,
    /// Optional failure recovery configuration.
    #[serde(default)]
    pub on_failure: Option<OnFailureConfig>,
    /// Shutdown timeout in seconds (default: 30).
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
    /// Process definitions for supervise.
    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,
}

impl BootstrapConfig {
    /// Loads a bootstrap configuration from a YAML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path)
            .with_context(|_| error::OpenConfigSnafu { filename: path.to_path_buf() })?;
        serde_yaml::from_slice(&data)
            .with_context(|_| error::ParseConfigSnafu { filename: path.to_path_buf() })
    }

    /// Converts to `ocelot_bootstrap::Config`.
    pub fn to_bootstrap_config(&self) -> ocelot_bootstrap::Config {
        let root = ocelot_bootstrap::RootConfig::from(self.root.clone());
        let modules = self.modules.clone().map(ocelot_bootstrap::ModuleConfig::from);
        let on_failure = self.on_failure.clone().map(ocelot_bootstrap::OnFailureConfig::from);
        ocelot_bootstrap::Config {
            root,
            modules,
            console: self.console.clone(),
            on_failure,
            shutdown_timeout: Duration::from_secs(self.shutdown_timeout_secs),
        }
    }

    /// Converts to `ocelot_supervise::OrchestratorConfig`.
    pub fn to_orchestrator_config(&self) -> ocelot_supervise::OrchestratorConfig {
        let supervisor_config = SuperviseConfig {
            version: "1.0".to_string(),
            log_level: tracing::Level::INFO,
            processes: self.processes.clone(),
            shutdown_timeout_secs: self.shutdown_timeout_secs,
        };

        ocelot_supervise::OrchestratorConfig {
            supervisors: supervisor_config.to_supervisors(),
            shutdown_timeout: Duration::from_secs(self.shutdown_timeout_secs),
        }
    }
}

impl From<RootConfig> for ocelot_bootstrap::RootConfig {
    fn from(config: RootConfig) -> Self {
        match config {
            RootConfig::Virtiofs { tag, overlay, options } => {
                Self::Virtiofs { tag, overlay: overlay.unwrap_or(false), options }
            }
            RootConfig::Block { device, fstype, overlay, options } => {
                Self::Block { device, fstype, overlay: overlay.unwrap_or(false), options }
            }
            RootConfig::NineP { tag, fstype, overlay, options } => {
                Self::NineP { tag, fstype, overlay: overlay.unwrap_or(false), options }
            }
        }
    }
}

impl From<ModulesConfig> for ocelot_bootstrap::ModuleConfig {
    fn from(config: ModulesConfig) -> Self { Self { dir: config.dir.clone(), list: config.list } }
}

impl From<OnFailureConfig> for ocelot_bootstrap::OnFailureConfig {
    fn from(config: OnFailureConfig) -> Self { Self { shell: config.shell } }
}

/// Root filesystem backend configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, tag = "type")]
pub enum RootConfig {
    /// Virtiofs shared filesystem.
    Virtiofs {
        /// Tag name for the virtiofs share.
        tag: String,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
    /// Block device (virtio-blk).
    Block {
        /// Device path (e.g., /dev/vda2).
        device: String,
        /// Filesystem type (e.g., ext4, xfs).
        fstype: String,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
    /// 9p shared filesystem.
    #[serde(rename = "9p")]
    NineP {
        /// Tag name for the 9p share.
        tag: String,
        /// Filesystem type (default: "9p").
        #[serde(default)]
        fstype: Option<String>,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
}

/// Kernel module loading configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModulesConfig {
    /// Directory containing kernel modules.
    #[serde(default)]
    pub dir: Option<String>,
    /// List of module names to load.
    #[serde(default)]
    pub list: Vec<String>,
}

/// Failure recovery configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnFailureConfig {
    /// Path to debug shell to spawn on failure.
    pub shell: Option<String>,
}

fn default_console() -> String { "console".to_string() }

const fn default_shutdown_timeout() -> u64 { 30 }
