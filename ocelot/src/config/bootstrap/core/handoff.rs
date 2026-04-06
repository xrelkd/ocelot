use serde::Deserialize;

use crate::config::{
    Error,
    bootstrap::{BootScriptConfig, BootstrapSuperviseConfig},
};

/// `HandoffMode`.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandoffMode {
    #[default]
    Supervise,
    Shell,
}

/// `HandoffConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffConfig {
    #[serde(default)]
    pub mode: HandoffMode,
    #[serde(default)]
    pub boot_script: Option<BootScriptConfig>,
    #[serde(default)]
    pub supervise: Option<BootstrapSuperviseConfig>,
    #[serde(default)]
    pub shell: Option<ShellConfig>,
}

impl HandoffConfig {
    pub fn validate(&self) -> Result<(), Error> {
        // Enforce mutual exclusivity.
        let has_supervise = self.supervise.is_some();
        let has_shell = self.shell.is_some();
        if has_supervise && has_shell {
            return Err(Error::InvalidConfig {
                message: "Cannot specify both shell and supervise".to_string(),
            });
        }
        if !has_supervise && !has_shell {
            return Err(Error::InvalidConfig {
                message: "Must specify either shell or supervise".to_string(),
            });
        }

        // Mode-specific validation.
        match self.mode {
            HandoffMode::Supervise => {
                let supervise = self.supervise.as_ref().expect("supervise is checked.");
                supervise.validate()?;
            }
            HandoffMode::Shell => {}
        }

        Ok(())
    }
}

impl From<HandoffConfig> for ocelot_bootstrap::Handoff {
    fn from(config: HandoffConfig) -> Self {
        let boot_script = config.boot_script.map(ocelot_bootstrap::BootScriptConfig::from);

        // Mode-specific validation.
        let mode = match config.mode {
            HandoffMode::Supervise => {
                let supervise = config.supervise.expect("supervise is checked.");
                ocelot_bootstrap::HandoffMode::Supervise(
                    ocelot_supervise::OrchestratorConfig::from(supervise),
                )
            }
            HandoffMode::Shell => {
                let shell = config.shell.expect("shell is checked.");
                ocelot_bootstrap::HandoffMode::Shell(ocelot_bootstrap::ShellConfig::from(shell))
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
