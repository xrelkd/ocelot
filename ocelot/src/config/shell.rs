use serde::Deserialize;

/// Shell execution configuration for debugging mode.
///
/// When configured, bootstrap spawns an interactive shell after `switch_root`
/// instead of executing the supervise orchestrator.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShellConfig {
    /// Path to the shell program to execute.
    pub program: String,
    /// Arguments to pass to the shell program.
    #[serde(default)]
    pub args: Vec<String>,
}

impl From<ShellConfig> for ocelot_bootstrap::ShellConfig {
    fn from(config: ShellConfig) -> Self { Self { program: config.program, args: config.args } }
}
