use serde::Deserialize;

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

/// Failure recovery configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnFailureConfig {
    /// Path to debug shell to spawn on failure.
    pub shell: Option<String>,
}

impl From<OnFailurePolicy> for ocelot_bootstrap::OnFailurePolicy {
    fn from(config: OnFailurePolicy) -> Self {
        match config {
            OnFailurePolicy::Warn => Self::Warn,
            OnFailurePolicy::Abort => Self::Abort,
        }
    }
}

impl From<OnFailureConfig> for ocelot_bootstrap::OnFailureConfig {
    fn from(config: OnFailureConfig) -> Self { Self { shell: config.shell } }
}
