use std::path::Path;

use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;
use tracing::Level;

use crate::config::{
    Error,
    bootstrap::core::{
        handoff::HandoffMode,
        phases::{PostSwitchConfig, PreSwitchConfig, SwitchRootConfig},
    },
};

/// Bootstrap configuration file structure.
///
/// This represents the YAML configuration for the bootstrap subcommand,
/// organized into three phases: `pre_switch`, `switch_root`, and `post_switch`.
#[serde_as]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootstrapConfig {
    /// Console device for output (default: "console").
    #[serde(default = "default_console")]
    pub console: String,
    /// Log level for supervise mode (default: "info").
    #[serde(default = "default_log_level")]
    #[serde_as(as = "DisplayFromStr")]
    pub log_level: Level,

    /// Pre-switch phase configuration.
    #[serde(default)]
    pub pre_switch: PreSwitchConfig,
    /// Switch-root phase configuration.
    pub switch_root: SwitchRootConfig,
    /// Post-switch phase configuration.
    #[serde(default)]
    pub post_switch: PostSwitchConfig,
}

impl BootstrapConfig {
    pub fn validate(&mut self) -> Result<(), Error> {
        self.validate_pre_switch()?;
        // TODO: implement and call validate_switch_root
        self.validate_post_switch()?;
        Ok(())
    }

    fn validate_pre_switch(&mut self) -> Result<(), Error> {
        if let Some(modules) = &mut self.pre_switch.modules {
            modules.validate()?;
        }
        Ok(())
    }

    fn validate_post_switch(&mut self) -> Result<(), Error> {
        if let Some(modules) = &mut self.post_switch.modules {
            modules.validate()?;
        }
        self.post_switch.handoff.validate()?;

        Ok(())
    }
}

impl BootstrapConfig {
    /// Loads a bootstrap configuration from a YAML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let data = std::fs::read(path).with_context(|_| crate::config::error::OpenConfigSnafu {
            filename: path.to_path_buf(),
        })?;
        let config: Self = serde_yaml::from_slice(&data).with_context(|_| {
            crate::config::error::ParseConfigSnafu { filename: path.to_path_buf() }
        })?;
        Ok(config)
    }

    /// Converts to `ocelot_bootstrap::Config`.
    pub fn to_bootstrap_config(&self) -> ocelot_bootstrap::Config {
        ocelot_bootstrap::Config::from(self.clone())
    }

    pub const fn handoff_mode(&self) -> HandoffMode { self.post_switch.handoff.mode }

    pub fn template_shell() -> Vec<u8> { include_bytes!("../templates/shell.yaml").to_vec() }

    pub fn template_supervise() -> Vec<u8> {
        include_bytes!("../templates/supervise.yaml").to_vec()
    }
}

impl From<BootstrapConfig> for ocelot_bootstrap::Config {
    fn from(config: BootstrapConfig) -> Self {
        let BootstrapConfig { console, pre_switch, switch_root, post_switch, .. } = config;
        Self {
            console,
            pre_switch: ocelot_bootstrap::PreSwitchPhase::from(pre_switch),
            switch_root: ocelot_bootstrap::SwitchRootPhase::from(switch_root),
            post_switch: ocelot_bootstrap::PostSwitchPhase::from(post_switch),
        }
    }
}

fn default_console() -> String { "console".to_string() }

const fn default_log_level() -> Level { Level::INFO }
