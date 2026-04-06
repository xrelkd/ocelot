use serde::Deserialize;

use crate::config::bootstrap::security::{apparmor::ApparmorConfig, selinux::SelinuxConfig};

/// `SecurityConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SecurityConfig {
    #[serde(default)]
    pub selinux: Option<SelinuxConfig>,
    #[serde(default)]
    pub apparmor: Option<ApparmorConfig>,
}

impl From<SecurityConfig> for ocelot_bootstrap::Security {
    fn from(config: SecurityConfig) -> Self {
        Self {
            selinux: config.selinux.map(ocelot_bootstrap::Selinux::from),
            apparmor: config.apparmor.map(ocelot_bootstrap::Apparmor::from),
        }
    }
}
