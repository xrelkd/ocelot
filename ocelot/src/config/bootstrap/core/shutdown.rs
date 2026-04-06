use std::time::Duration;

use serde::Deserialize;

/// `ShutdownConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShutdownConfig {
    #[serde(default)]
    pub timeout_secs: u64,
    #[serde(default = "default_true")]
    pub sync: bool,
    #[serde(default = "default_true")]
    pub umount_all: bool,
}

impl From<ShutdownConfig> for ocelot_bootstrap::Shutdown {
    fn from(config: ShutdownConfig) -> Self {
        Self {
            timeout: Duration::from_secs(config.timeout_secs),
            sync: config.sync,
            umount_all: config.umount_all,
        }
    }
}

const fn default_true() -> bool { true }
