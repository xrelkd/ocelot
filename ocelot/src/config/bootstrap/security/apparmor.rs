use serde::Deserialize;

/// `AppArmor` configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApparmorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub profile: Option<String>,
}

impl From<ApparmorConfig> for ocelot_bootstrap::Apparmor {
    fn from(config: ApparmorConfig) -> Self {
        Self { enabled: config.enabled, profile: config.profile }
    }
}
