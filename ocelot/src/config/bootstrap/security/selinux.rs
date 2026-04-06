use serde::Deserialize;

/// `SELinux` configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelinuxConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub policy: Option<String>,
}

impl From<SelinuxConfig> for ocelot_bootstrap::Selinux {
    fn from(config: SelinuxConfig) -> Self {
        Self { enabled: config.enabled, policy: config.policy }
    }
}
