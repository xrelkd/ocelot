use serde::Deserialize;

/// `ShutdownConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShutdownConfig {
    #[serde(default)]
    pub timeout: u32,
    #[serde(default = "default_true")]
    pub sync: bool,
    #[serde(default = "default_true")]
    pub umount_all: bool,
}

impl From<ShutdownConfig> for ocelot_bootstrap::Shutdown {
    fn from(config: ShutdownConfig) -> Self {
        Self { timeout: config.timeout, sync: config.sync, umount_all: config.umount_all }
    }
}

const fn default_true() -> bool { true }
