use std::time::Duration;

/// Bootstrap-specific configuration for early boot initialization.
///
/// This config covers only the initramfs boot phase: root filesystem,
/// kernel modules, console, and failure recovery. Process supervision
/// configuration is handled separately by the CLI and passed as
/// `ocelot_supervise::OrchestratorConfig`.
#[derive(Clone, Debug)]
pub struct Config {
    pub root: RootConfig,
    pub modules: Option<ModuleConfig>,
    pub console: String,
    pub on_failure: Option<OnFailureConfig>,
    pub shutdown_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: RootConfig::Virtiofs { tag: String::new(), overlay: false, options: None },
            modules: None,
            console: "console".to_string(),
            on_failure: None,
            shutdown_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Clone, Debug)]
pub enum RootConfig {
    Virtiofs { tag: String, overlay: bool, options: Option<String> },
    Block { device: String, fstype: String, overlay: bool, options: Option<String> },
    NineP { tag: String, fstype: Option<String>, overlay: bool, options: Option<String> },
}

impl RootConfig {
    #[must_use]
    pub const fn overlay(&self) -> bool {
        match self {
            Self::Virtiofs { overlay, .. }
            | Self::Block { overlay, .. }
            | Self::NineP { overlay, .. } => *overlay,
        }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Self::Block { device, .. } => device,
            Self::Virtiofs { tag, .. } | Self::NineP { tag, .. } => tag,
        }
    }

    #[must_use]
    pub fn fstype(&self) -> &str {
        match self {
            Self::Virtiofs { .. } => "virtiofs",
            Self::Block { fstype, .. } => fstype,
            Self::NineP { fstype, .. } => fstype.as_deref().unwrap_or("9p"),
        }
    }

    #[must_use]
    pub fn mount_options(&self) -> Option<&str> {
        match self {
            Self::Virtiofs { options, .. }
            | Self::Block { options, .. }
            | Self::NineP { options, .. } => options.as_deref(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ModuleConfig {
    pub dir: Option<String>,
    pub list: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct OnFailureConfig {
    pub shell: Option<String>,
}
