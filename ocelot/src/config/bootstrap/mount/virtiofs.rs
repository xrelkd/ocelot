use serde::Deserialize;

/// Configuration for an extra virtiofs mount.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VirtiofsMountConfig {
    /// Tag name for the virtiofs share.
    pub tag: String,
    /// Mount point path (relative to new root).
    pub path: String,
    /// Whether to set up an overlayfs on top of this mount.
    #[serde(default)]
    pub with_overlay: Option<bool>,
    /// Additional mount options.
    #[serde(default)]
    pub options: Option<String>,
}

impl From<VirtiofsMountConfig> for ocelot_bootstrap::VirtiofsMount {
    fn from(config: VirtiofsMountConfig) -> Self {
        Self {
            tag: config.tag,
            path: config.path,
            with_overlay: config.with_overlay.unwrap_or(false),
            options: config.options,
        }
    }
}
