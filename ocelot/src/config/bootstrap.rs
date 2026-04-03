use std::{collections::HashMap, path::Path, time::Duration};

use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;
use tracing::Level;

use crate::config::{
    Error, ProcessConfig, SuperviseConfig, error, error::ValidationError, shell::ShellConfig,
};

/// Bootstrap configuration file structure.
///
/// This represents the YAML configuration for the bootstrap subcommand,
/// which combines bootstrap-specific options with either shell or supervise
/// execution mode.
#[serde_as]
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
    /// Log level for supervise mode (default: "info").
    #[serde(default = "default_log_level")]
    #[serde_as(as = "DisplayFromStr")]
    pub log_level: Level,
    /// Optional failure recovery configuration.
    #[serde(default)]
    pub on_failure: Option<OnFailureConfig>,
    /// Shutdown timeout in seconds (default: 30).
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
    /// Environment variables to set before executing shell or supervise.
    #[serde(default)]
    pub environment_variables: Vec<(String, String)>,
    /// Working directory to change to before executing shell or supervise.
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Execution mode: shell for debugging, or supervise for normal operation.
    #[serde(flatten)]
    pub mode: ExecutionMode,
}

/// Execution mode for bootstrap, mutually exclusive.
///
/// Either shell mode (for debugging) or supervise mode (normal operation).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[serde(tag = "mode")]
pub enum ExecutionMode {
    /// Shell execution mode for debugging.
    Shell {
        /// Shell configuration.
        #[serde(flatten)]
        config: ShellConfig,
    },
    /// Supervise mode with process definitions.
    Supervise {
        /// Process definitions for supervise.
        #[serde(default)]
        processes: HashMap<String, ProcessConfig>,
    },
}

impl ExecutionMode {
    /// Returns the shell config if in shell mode, None otherwise.
    #[must_use]
    pub const fn shell_config(&self) -> Option<&ShellConfig> {
        match self {
            Self::Shell { config } => Some(config),
            Self::Supervise { .. } => None,
        }
    }

    /// Returns the processes if in supervise mode, None otherwise.
    #[must_use]
    pub const fn processes(&self) -> Option<&HashMap<String, ProcessConfig>> {
        match self {
            Self::Shell { .. } => None,
            Self::Supervise { processes } => Some(processes),
        }
    }
}

impl BootstrapConfig {
    /// Loads a bootstrap configuration from a YAML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path)
            .with_context(|_| error::OpenConfigSnafu { filename: path.to_path_buf() })?;
        let config: Self = serde_yaml::from_slice(&data)
            .with_context(|_| error::ParseConfigSnafu { filename: path.to_path_buf() })?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        self.validate_environment_variables()?;
        Ok(())
    }

    fn validate_environment_variables(&self) -> Result<(), ValidationError> {
        let (_seen, duplicates): (_, Vec<_>) = self.environment_variables.iter().fold(
            (std::collections::HashSet::new(), Vec::new()),
            |(mut seen, mut dups), (key, _)| {
                if !seen.insert(key) {
                    dups.push(key.clone());
                }
                (seen, dups)
            },
        );
        if !duplicates.is_empty() {
            return Err(ValidationError::BootstrapDuplicateEnvironmentVariables {
                variables: duplicates,
            });
        }
        Ok(())
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
            environment_variables: self.environment_variables.clone(),
            working_directory: self.working_directory.clone(),
        }
    }

    /// Converts shell config to `ocelot_bootstrap::ShellConfig`.
    ///
    /// Returns `None` if not in shell mode.
    #[must_use]
    pub fn to_shell_config(&self) -> Option<ocelot_bootstrap::ShellConfig> {
        self.mode.shell_config().map(|c| c.clone().into())
    }

    /// Converts to `ocelot_supervise::OrchestratorConfig`.
    ///
    /// Returns `None` if in shell mode.
    #[must_use]
    pub fn to_orchestrator_config(&self) -> Option<ocelot_supervise::OrchestratorConfig> {
        let processes = self.mode.processes()?;

        let supervisor_config = SuperviseConfig {
            version: "1.0".to_string(),
            log_level: Level::INFO,
            processes: processes.clone(),
            shutdown_timeout_secs: self.shutdown_timeout_secs,
        };

        Some(ocelot_supervise::OrchestratorConfig {
            supervisors: supervisor_config.to_supervisors(),
            shutdown_timeout: Duration::from_secs(self.shutdown_timeout_secs),
        })
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

const fn default_log_level() -> Level { Level::INFO }

const fn default_shutdown_timeout() -> u64 { 30 }
