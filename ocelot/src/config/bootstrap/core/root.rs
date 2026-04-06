use serde::Deserialize;

/// Root filesystem backend configuration.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, tag = "type")]
pub enum RootConfig {
    /// Virtiofs shared filesystem.
    Virtiofs {
        /// Tag name for the virtiofs share.
        tag: String,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
    /// Block device (virtio-blk).
    Block {
        /// Device path (e.g., /dev/vda2).
        device: String,
        /// Filesystem type (e.g., ext4, xfs).
        fstype: String,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
    /// 9p shared filesystem.
    #[serde(rename = "9p")]
    NineP {
        /// Tag name for the 9p share.
        tag: String,
        /// Filesystem type (default: "9p").
        #[serde(default)]
        fstype: Option<String>,
        /// Whether to use overlay filesystem.
        #[serde(default)]
        overlay: Option<bool>,
        /// Additional mount options.
        #[serde(default)]
        options: Option<String>,
    },
}

impl From<RootConfig> for ocelot_bootstrap::RootConfig {
    fn from(config: RootConfig) -> Self {
        match config {
            RootConfig::Virtiofs { tag, overlay, options } => {
                Self::Virtiofs { tag, overlay: overlay.unwrap_or(false), options }
            }
            RootConfig::Block { device, fstype, overlay, options } => Self::Block {
                device: device.into(),
                fstype,
                overlay: overlay.unwrap_or(false),
                options,
            },
            RootConfig::NineP { tag, fstype, overlay, options } => {
                Self::NineP { tag, fstype, overlay: overlay.unwrap_or(false), options }
            }
        }
    }
}
