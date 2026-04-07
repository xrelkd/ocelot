use std::time::Duration;

use serde::Deserialize;

use crate::config::bootstrap::{mount::MountFailurePolicy, supervise::policy::OnFailurePolicy};

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

/// `HookSpecConfig`: Hook specification config (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HookSpecConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default)]
    pub on_failure: MountFailurePolicy,
}

impl From<HookSpecConfig> for ocelot_bootstrap::HookSpec {
    fn from(config: HookSpecConfig) -> Self {
        Self {
            name: config.name,
            command: config.command,
            arguments: config.arguments,
            timeout: Duration::from_secs(config.timeout_secs),
            on_failure: match config.on_failure {
                MountFailurePolicy::Warn => ocelot_bootstrap::MountFailurePolicy::Warn,
                MountFailurePolicy::Abort => ocelot_bootstrap::MountFailurePolicy::Abort,
                MountFailurePolicy::Retry => ocelot_bootstrap::MountFailurePolicy::Retry,
            },
        }
    }
}
