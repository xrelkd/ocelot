use std::{collections::HashMap, path::Path, time::Duration};

use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;
use tracing::Level;

use crate::config::{
    Error, ProcessConfig, SuperviseConfig, error, error::ValidationError,
    shell::BootstrapShellConfig,
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
    /// Shell mode configuration (mutually exclusive with supervise).
    #[serde(default)]
    pub shell: Option<BootstrapShellConfig>,
    /// Supervise mode configuration (mutually exclusive with shell).
    #[serde(default)]
    pub supervise: Option<BootstrapSuperviseConfig>,
    /// Extra virtiofs mounts to set up after the root filesystem.
    #[serde(default)]
    pub extra_virtiofs_mounts: Vec<VirtiofsMountConfig>,
    /// Symlinks to create after `switch_root`.
    #[serde(default)]
    pub symlinks: Vec<SymlinkConfig>,
    /// Optional boot script to execute before handoff.
    #[serde(default)]
    pub boot_script: Option<BootScriptConfig>,
}

/// Bootstrap supervise configuration wrapper.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootstrapSuperviseConfig {
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
        let mut config: Self = serde_yaml::from_slice(&data)
            .with_context(|_| error::ParseConfigSnafu { filename: path.to_path_buf() })?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&mut self) -> Result<(), Error> {
        self.validate_environment_variables()?;
        self.validate_mode_exclusivity()?;
        self.validate_module_dependencies()?;
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

    fn validate_mode_exclusivity(&self) -> Result<(), Error> {
        let has_shell = self.shell.is_some();
        let has_supervise = self.supervise.is_some();

        if has_shell && has_supervise {
            return Err(Error::InvalidConfig {
                message: "Cannot specify both 'shell' and 'supervise' modes. They are mutually \
                          exclusive."
                    .to_string(),
            });
        }

        if !has_shell && !has_supervise {
            return Err(Error::InvalidConfig {
                message: "Must specify either 'shell' or 'supervise' mode.".to_string(),
            });
        }

        Ok(())
    }

    fn validate_module_dependencies(&mut self) -> Result<(), Error> {
        let Some(modules) = &mut self.modules else {
            return Ok(());
        };

        match modules {
            ModulesConfig::List { dir: _, names, dep_file_path } => {
                let Some(dep_path) = dep_file_path else {
                    return Ok(());
                };

                let dep_path_clone = dep_path.clone();
                let data = std::fs::read(dep_path).map_err(|source| {
                    Error::ParseModuleDependencyFile { path: dep_path_clone, source }
                })?;

                let dep_map = super::modules_dep::parse_dep_file(&data);
                let sorted = super::modules_dep::resolve_module_order(&dep_map, names)
                    .map_err(|e| Error::Validate { source: e })?;

                *names = sorted;
            }
            ModulesConfig::Scan { dir: _, dep_file_path, names } => {
                let dep_path_clone = dep_file_path.clone();
                let data = std::fs::read(dep_file_path).map_err(|source| {
                    Error::ParseModuleDependencyFile { path: dep_path_clone, source }
                })?;

                let dep_map = super::modules_dep::parse_dep_file(&data);
                let targets = names.clone().unwrap_or_default();
                let sorted = super::modules_dep::resolve_module_order(&dep_map, &targets)
                    .map_err(|e| Error::Validate { source: e })?;

                *names = Some(sorted);
            }
        }

        Ok(())
    }

    /// Converts to `ocelot_bootstrap::Config`.
    pub fn to_bootstrap_config(&self) -> ocelot_bootstrap::Config {
        let root = ocelot_bootstrap::RootConfig::from(self.root.clone());
        let modules = self.modules.clone().map(ocelot_bootstrap::ModulesConfig::from);
        let on_failure = self.on_failure.clone().map(ocelot_bootstrap::OnFailureConfig::from);
        let extra_virtiofs_mounts = self
            .extra_virtiofs_mounts
            .iter()
            .cloned()
            .map(ocelot_bootstrap::VirtiofsMount::from)
            .collect();
        let symlinks =
            self.symlinks.iter().cloned().map(ocelot_bootstrap::SymlinkSpec::from).collect();
        let boot_script = self.boot_script.clone().map(ocelot_bootstrap::BootScriptConfig::from);
        ocelot_bootstrap::Config {
            root,
            modules,
            console: self.console.clone(),
            on_failure,
            shutdown_timeout: Duration::from_secs(self.shutdown_timeout_secs),
            environment_variables: self.environment_variables.clone(),
            working_directory: self.working_directory.clone(),
            extra_virtiofs_mounts,
            symlinks,
            boot_script,
        }
    }

    /// Converts shell config to `ocelot_bootstrap::ShellConfig`.
    ///
    /// Returns `None` if not in shell mode.
    #[must_use]
    pub fn to_shell_config(&self) -> Option<ocelot_bootstrap::ShellConfig> {
        self.shell.as_ref().map(|c| c.clone().into())
    }

    /// Converts to `ocelot_supervise::OrchestratorConfig`.
    ///
    /// Returns `None` if in shell mode.
    #[must_use]
    pub fn to_orchestrator_config(&self) -> Option<ocelot_supervise::OrchestratorConfig> {
        let processes = self.supervise.as_ref()?.processes.clone();

        let supervisor_config = SuperviseConfig {
            version: "1.0".to_string(),
            log_level: Level::INFO,
            processes,
            shutdown_timeout_secs: self.shutdown_timeout_secs,
        };

        Some(ocelot_supervise::OrchestratorConfig {
            supervisors: supervisor_config.to_supervisors(),
            shutdown_timeout: Duration::from_secs(self.shutdown_timeout_secs),
        })
    }

    pub fn template_shell() -> Vec<u8> { include_bytes!("templates/bootstrap/shell.yaml").to_vec() }

    pub fn template_supervise() -> Vec<u8> {
        include_bytes!("templates/bootstrap/supervise.yaml").to_vec()
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

impl From<ModulesConfig> for ocelot_bootstrap::ModulesConfig {
    fn from(config: ModulesConfig) -> Self {
        match config {
            ModulesConfig::List { dir, names, .. } => Self::List { dir, names },
            ModulesConfig::Scan { dir: _, names, .. } => {
                let resolved_names = names.unwrap_or_default();
                Self::List { dir: None, names: resolved_names }
            }
        }
    }
}

impl From<OnFailureConfig> for ocelot_bootstrap::OnFailureConfig {
    fn from(config: OnFailureConfig) -> Self { Self { shell: config.shell } }
}

impl From<VirtiofsMountConfig> for ocelot_bootstrap::VirtiofsMount {
    fn from(config: VirtiofsMountConfig) -> Self {
        Self {
            tag: config.tag,
            path: config.path,
            with_overlay: config.with_overlay.unwrap_or(false),
            options: config.options,
        }
    }
}

impl From<SymlinkConfig> for ocelot_bootstrap::SymlinkSpec {
    fn from(config: SymlinkConfig) -> Self { Self { source: config.source, target: config.target } }
}

impl From<BootScriptConfig> for ocelot_bootstrap::BootScriptConfig {
    fn from(config: BootScriptConfig) -> Self {
        Self {
            command: config.command,
            arguments: config.arguments,
            on_failure: config.on_failure.into(),
            working_directory: config.working_directory,
        }
    }
}

impl From<OnFailurePolicy> for ocelot_bootstrap::OnFailurePolicy {
    fn from(config: OnFailurePolicy) -> Self {
        match config {
            OnFailurePolicy::Warn => Self::Warn,
            OnFailurePolicy::Abort => Self::Abort,
        }
    }
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
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, tag = "mode")]
pub enum ModulesConfig {
    /// Load specific modules by name.
    List {
        /// Directory containing kernel modules (defaults to /lib/modules).
        #[serde(default)]
        dir: Option<String>,
        /// List of module names to load.
        names: Vec<String>,
        /// Optional path to a modules.dep file for dependency resolution.
        #[serde(default)]
        dep_file_path: Option<String>,
    },
    /// Scan directory for all .ko/.ko.xz/.ko.gz files and load each.
    Scan {
        /// Directory to scan for kernel modules.
        dir: String,
        /// Path to a modules.dep file for dependency resolution.
        dep_file_path: String,
        /// Optional list of module names to filter which modules to load.
        #[serde(default)]
        names: Option<Vec<String>>,
    },
}

/// Failure recovery configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnFailureConfig {
    /// Path to debug shell to spawn on failure.
    pub shell: Option<String>,
}

/// Configuration for an extra virtiofs mount.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VirtiofsMountConfig {
    /// Tag name for the virtiofs share.
    pub tag: String,
    /// Mount point path (relative to new root).
    pub path: String,
    /// Whether to set up an overlayfs on top of this mount.
    #[serde(default)]
    pub with_overlay: Option<bool>,
    /// Additional mount options.
    #[serde(default)]
    pub options: Option<String>,
}

/// Configuration for a symlink to create during boot.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymlinkConfig {
    /// The target path the symlink should point to.
    pub source: String,
    /// The path where the symlink should be created.
    pub target: String,
}

/// Configuration for boot script execution.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootScriptConfig {
    /// The command to execute.
    pub command: String,
    /// Arguments for the command.
    #[serde(default)]
    pub arguments: Vec<String>,
    /// Policy for handling non-zero exit codes (default: warn).
    #[serde(default)]
    pub on_failure: OnFailurePolicy,
    /// Working directory for script execution.
    #[serde(default)]
    pub working_directory: Option<String>,
}

/// Policy for handling boot script failures.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OnFailurePolicy {
    /// Log a warning and continue the boot process.
    #[serde(rename = "warn")]
    #[default]
    Warn,
    /// Return an error and abort the boot process.
    #[serde(rename = "abort")]
    Abort,
}

fn default_console() -> String { "console".to_string() }

const fn default_log_level() -> Level { Level::INFO }

const fn default_shutdown_timeout() -> u64 { 30 }
