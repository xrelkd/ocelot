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
    pub modules: Option<ModulesConfig>,
    pub console: String,
    pub on_failure: Option<OnFailureConfig>,
    pub shutdown_timeout: Duration,
    pub environment_variables: Vec<(String, String)>,
    pub working_directory: Option<String>,
    pub extra_virtiofs_mounts: Vec<VirtiofsMount>,
    pub symlinks: Vec<SymlinkSpec>,
    pub boot_script: Option<BootScriptConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root: RootConfig::Virtiofs { tag: String::new(), overlay: false, options: None },
            modules: None,
            console: "console".to_string(),
            on_failure: None,
            shutdown_timeout: Duration::from_secs(30),
            environment_variables: Vec::new(),
            working_directory: None,
            extra_virtiofs_mounts: Vec::new(),
            symlinks: Vec::new(),
            boot_script: None,
        }
    }
}

/// Root filesystem backend configuration.
///
/// Supports three filesystem types for the root partition:
/// - Virtiofs: shared filesystem via virtio-fs (recommended for QEMU)
/// - Block: raw block device (virtio-blk)
/// - `NineP`: 9p2000.L network filesystem
#[derive(Clone, Debug)]
pub enum RootConfig {
    /// Virtiofs shared filesystem.
    Virtiofs {
        /// Tag name for the virtiofs share.
        tag: String,
        /// Whether to use overlay filesystem on top.
        overlay: bool,
        /// Additional mount options.
        options: Option<String>,
    },
    /// Block device (virtio-blk).
    Block {
        /// Device path (e.g., `/dev/vda2`).
        device: String,
        /// Filesystem type (e.g., `ext4`, `xfs`).
        fstype: String,
        /// Whether to use overlay filesystem on top.
        overlay: bool,
        /// Additional mount options.
        options: Option<String>,
    },
    /// 9p shared filesystem.
    NineP {
        /// Tag name for the 9p share.
        tag: String,
        /// Filesystem type (default: `9p`).
        fstype: Option<String>,
        /// Whether to use overlay filesystem on top.
        overlay: bool,
        /// Additional mount options.
        options: Option<String>,
    },
}

impl RootConfig {
    /// Returns whether overlay filesystem is enabled for this root config.
    #[must_use]
    pub const fn overlay(&self) -> bool {
        match self {
            Self::Virtiofs { overlay, .. }
            | Self::Block { overlay, .. }
            | Self::NineP { overlay, .. } => *overlay,
        }
    }

    /// Returns the mount source (tag or device path).
    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Self::Block { device, .. } => device,
            Self::Virtiofs { tag, .. } | Self::NineP { tag, .. } => tag,
        }
    }

    /// Returns the filesystem type for mounting.
    #[must_use]
    pub fn fstype(&self) -> &str {
        match self {
            Self::Virtiofs { .. } => "virtiofs",
            Self::Block { fstype, .. } => fstype,
            Self::NineP { fstype, .. } => fstype.as_deref().unwrap_or("9p"),
        }
    }

    /// Returns additional mount options, if configured.
    #[must_use]
    pub fn mount_options(&self) -> Option<&str> {
        match self {
            Self::Virtiofs { options, .. }
            | Self::Block { options, .. }
            | Self::NineP { options, .. } => options.as_deref(),
        }
    }
}

/// Configuration for kernel module loading.
///
/// Supports two mutually exclusive modes:
/// - `List`: Load specific modules by name from a directory
/// - `Scan`: Auto-discover and load all `.ko`/`.ko.xz`/`.ko.gz` files from a
///   directory
///
/// # Dependency Ordering
///
/// The `names` list in [`ModulesConfig::List`] is assumed to be in correct
/// dependency order — dependencies before dependents. This ordering is
/// validated by the ocelot binary's config layer when a `modules.dep` file
/// is provided. If no dependency file is configured, the user is responsible
/// for specifying the correct order.
#[derive(Clone, Debug)]
pub enum ModulesConfig {
    /// Load specific modules by name.
    ///
    /// When `dir` is `None`, defaults to `/lib/modules`.
    ///
    /// # Ordering
    /// Modules are loaded in the order specified in `names`. This list is
    /// expected to be in correct dependency order (dependencies before
    /// dependents), as validated by the ocelot config layer when a
    /// `modules.dep` file is provided.
    List { dir: Option<String>, names: Vec<String> },
    /// Scan directory for all `.ko`/`.ko.xz`/`.ko.gz` files and load each.
    ///
    /// # Ordering
    /// Modules are loaded in the order specified in `names` (populated by
    /// the ocelot config layer via dependency resolution from a
    /// `modules.dep` file).
    Scan { dir: String, names: Option<Vec<String>> },
}

impl Default for ModulesConfig {
    fn default() -> Self { Self::List { dir: None, names: Vec::new() } }
}

/// Legacy kernel module loading configuration (flat struct).
///
/// This is kept for backward compatibility. New code should use
/// [`ModulesConfig`] instead.
#[derive(Clone, Debug, Default)]
pub struct ModuleConfig {
    /// Directory containing kernel modules.
    pub dir: Option<String>,
    /// List of module names to load.
    pub list: Vec<String>,
}

/// Failure recovery configuration.
///
/// When a boot stage fails, this config determines whether to spawn
/// a debug shell for manual intervention.
#[derive(Clone, Debug, Default)]
pub struct OnFailureConfig {
    /// Path to debug shell to spawn on failure.
    pub shell: Option<String>,
}

/// Configuration for shell execution mode.
///
/// When configured, bootstrap spawns an interactive shell after `switch_root`
/// instead of executing the supervise orchestrator. This mode is mutually
/// exclusive with supervise mode.
#[derive(Clone, Debug)]
pub struct ShellConfig {
    /// Shell program to execute.
    pub program: String,
    /// Arguments for the shell program.
    pub arguments: Vec<String>,
}

/// Configuration for an extra virtiofs mount.
///
/// Represents a single virtiofs share to mount in addition to the root
/// filesystem.
#[derive(Clone, Debug, Default)]
pub struct VirtiofsMount {
    /// Tag name for the virtiofs share.
    pub tag: String,
    /// Mount point path (relative to new root).
    pub path: String,
    /// Whether to set up an overlayfs on top of this mount.
    pub with_overlay: bool,
    /// Additional mount options.
    pub options: Option<String>,
}

/// Specification for a symlink to create during boot.
#[derive(Clone, Debug, Default)]
pub struct SymlinkSpec {
    /// The target path the symlink should point to.
    pub source: String,
    /// The path where the symlink should be created.
    pub target: String,
}

/// Configuration for boot script execution.
///
/// Optionally runs a script before handing off to the supervise orchestrator
/// or spawning a shell.
#[derive(Clone, Debug, Default)]
pub struct BootScriptConfig {
    /// The command to execute.
    pub command: String,
    /// Arguments for the command.
    pub arguments: Vec<String>,
    /// Policy for handling non-zero exit codes.
    pub on_failure: OnFailurePolicy,
    /// Working directory for script execution.
    pub working_directory: Option<String>,
}

/// Policy for handling boot script failures.
#[derive(Clone, Debug, Default)]
pub enum OnFailurePolicy {
    /// Log a warning and continue the boot process.
    #[default]
    Warn,
    /// Return an error and abort the boot process.
    Abort,
}

#[cfg(test)]
mod tests {
    use super::{ModuleConfig, ModulesConfig, OnFailureConfig, OnFailurePolicy, RootConfig};

    #[test]
    fn test_root_config_virtiofs_overlay() {
        let config =
            RootConfig::Virtiofs { tag: "rootfs".to_string(), overlay: true, options: None };
        assert!(config.overlay());
        assert_eq!(config.source(), "rootfs");
        assert_eq!(config.fstype(), "virtiofs");
        assert!(config.mount_options().is_none());
    }

    #[test]
    fn test_root_config_virtiofs_no_overlay() {
        let config = RootConfig::Virtiofs {
            tag: "rootfs".to_string(),
            overlay: false,
            options: Some("ro".to_string()),
        };
        assert!(!config.overlay());
        assert_eq!(config.source(), "rootfs");
        assert_eq!(config.fstype(), "virtiofs");
        assert_eq!(config.mount_options(), Some("ro"));
    }

    #[test]
    fn test_root_config_block() {
        let config = RootConfig::Block {
            device: "/dev/vda2".to_string(),
            fstype: "ext4".to_string(),
            overlay: true,
            options: None,
        };
        assert!(config.overlay());
        assert_eq!(config.source(), "/dev/vda2");
        assert_eq!(config.fstype(), "ext4");
        assert!(config.mount_options().is_none());
    }

    #[test]
    fn test_root_config_ninep_default_fstype() {
        let config = RootConfig::NineP {
            tag: "rootfs".to_string(),
            fstype: None,
            overlay: false,
            options: None,
        };
        assert!(!config.overlay());
        assert_eq!(config.source(), "rootfs");
        assert_eq!(config.fstype(), "9p");
        assert!(config.mount_options().is_none());
    }

    #[test]
    fn test_root_config_ninep_custom_fstype() {
        let config = RootConfig::NineP {
            tag: "rootfs".to_string(),
            fstype: Some("9p2000.L".to_string()),
            overlay: false,
            options: Some("trans=virtio".to_string()),
        };
        assert!(!config.overlay());
        assert_eq!(config.fstype(), "9p2000.L");
        assert_eq!(config.mount_options(), Some("trans=virtio"));
    }

    #[test]
    fn test_module_config_default() {
        let config = ModulesConfig::default();
        match config {
            ModulesConfig::List { dir, names } => {
                assert!(dir.is_none());
                assert!(names.is_empty());
            }
            ModulesConfig::Scan { .. } => panic!("expected List variant"),
        }
    }

    #[test]
    fn test_module_config_legacy_default() {
        let config = ModuleConfig::default();
        assert!(config.dir.is_none());
        assert!(config.list.is_empty());
    }

    #[test]
    fn test_on_failure_config_default() {
        let config = OnFailureConfig::default();
        assert!(config.shell.is_none());
    }

    #[test]
    fn test_on_failure_policy_default() {
        let policy = OnFailurePolicy::default();
        assert!(matches!(policy, OnFailurePolicy::Warn));
    }

    #[test]
    fn test_on_failure_policy_variants() {
        assert!(matches!(OnFailurePolicy::Warn, OnFailurePolicy::Warn));
        assert!(matches!(OnFailurePolicy::Abort, OnFailurePolicy::Abort));
    }
}
