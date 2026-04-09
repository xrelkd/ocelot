use serde::Deserialize;

use crate::config::{
    Error,
    bootstrap::{BootScriptConfig, BootstrapSuperviseConfig},
};

/// `HandoffMode`.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "mode")]
pub enum HandoffMode {
    Supervise {
        #[serde(flatten)]
        config: BootstrapSuperviseConfig,
    },
    Shell {
        #[serde(flatten)]
        config: ShellConfig,
    },
    Exec {
        #[serde(flatten)]
        config: ExecConfig,
    },
}

/// `HandoffConfig`.
#[derive(Clone, Debug, Deserialize)]
pub struct HandoffConfig {
    #[serde(flatten)]
    pub mode: HandoffMode,
    #[serde(default)]
    pub boot_script: Option<BootScriptConfig>,
}

impl HandoffConfig {
    pub fn validate(&self) -> Result<(), Error> {
        // Mode-specific validation.
        match &self.mode {
            HandoffMode::Supervise { config } => config.validate()?,
            HandoffMode::Shell { .. } | HandoffMode::Exec { .. } => {}
        }

        Ok(())
    }
}

impl From<HandoffConfig> for ocelot_bootstrap::Handoff {
    fn from(config: HandoffConfig) -> Self {
        let boot_script = config.boot_script.map(ocelot_bootstrap::BootScriptConfig::from);
        let mode = match config.mode {
            HandoffMode::Supervise { config: supervise_config } => {
                ocelot_bootstrap::HandoffMode::Supervise(
                    ocelot_supervise::OrchestratorConfig::from(supervise_config),
                )
            }
            HandoffMode::Shell { config: shell_config } => ocelot_bootstrap::HandoffMode::Shell(
                ocelot_bootstrap::ShellConfig::from(shell_config),
            ),
            HandoffMode::Exec { config: exec_config } => {
                ocelot_bootstrap::HandoffMode::Exec(ocelot_bootstrap::ExecConfig::from(exec_config))
            }
        };
        Self { mode, boot_script }
    }
}

/// `ShellConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShellConfig {
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

impl From<ShellConfig> for ocelot_bootstrap::ShellConfig {
    fn from(ShellConfig { program, arguments }: ShellConfig) -> Self { Self { program, arguments } }
}

/// `ExecConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecConfig {
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

impl From<ExecConfig> for ocelot_bootstrap::ExecConfig {
    fn from(ExecConfig { program, arguments }: ExecConfig) -> Self { Self { program, arguments } }
}
