use std::collections::HashMap;

use serde::Deserialize;

/// `SysctlConfig`.
pub type SysctlConfig = HashMap<String, String>;

/// `ClockConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClockConfig {
    #[serde(default = "default_true")]
    pub rtc_sync: bool,
}

impl From<ClockConfig> for ocelot_bootstrap::Clock {
    fn from(config: ClockConfig) -> Self { Self { rtc_sync: config.rtc_sync } }
}

/// `TmpfileConfig`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TmpfileConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub r#type: String,
}

impl From<TmpfileConfig> for ocelot_bootstrap::Tmpfile {
    fn from(config: TmpfileConfig) -> Self {
        Self { path: config.path.into(), mode: config.mode, r#type: config.r#type }
    }
}

/// Configuration for a symlink to create during boot.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymlinkConfig {
    /// The target path the symlink should point to.
    pub source: String,
    /// The path where the symlink should be created.
    pub target: String,
}

impl From<SymlinkConfig> for ocelot_bootstrap::Symlink {
    fn from(config: SymlinkConfig) -> Self {
        Self { source: config.source.into(), target: config.target.into() }
    }
}

const fn default_true() -> bool { true }
