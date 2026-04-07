use std::{collections::HashMap, time::Duration};

/// Bootstrap-specific configuration for early boot initialization.
#[derive(Clone, Debug)]
pub struct Config {
    /// Pre-switch phase configuration.
    pub pre_switch: PreSwitchPhase,
    /// Switch-root phase configuration.
    pub switch_root: SwitchRootPhase,
    /// Post-switch phase configuration.
    pub post_switch: PostSwitchPhase,
    /// Console device for shell output.
    pub console: String,
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
        device: std::path::PathBuf,
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
    /// # Panics
    /// This function never panics.
    #[must_use]
    pub fn source(&self) -> &str {
        match self {
            Self::Block { device, .. } => device.to_str().expect("device path must be valid UTF-8"),
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
    List {
        /// Directory containing kernel modules.
        dir: Option<std::path::PathBuf>,
        names: Vec<String>,
    },
    /// Scan directory for all `.ko`/`.ko.xz`/`.ko.gz` files and load each.
    ///
    /// # Ordering
    /// Modules are loaded in the order specified in `names` (populated by
    /// the ocelot config layer via dependency resolution from a
    /// `modules.dep` file).
    Scan {
        /// Directory to scan for kernel modules.
        dir: std::path::PathBuf,
        names: Option<Vec<String>>,
    },
}

impl Default for ModulesConfig {
    fn default() -> Self { Self::List { dir: None, names: Vec::new() } }
}

/// Switch-root phase configuration.
#[derive(Clone, Debug, Default)]
pub struct SwitchRootPhase {
    /// Method to use for switching root.
    pub method: SwitchRootMethod,
    /// Old root directory to clean up.
    pub old_root_dir: Option<String>,
    /// Whether to cleanup the old root.
    pub cleanup_old_root: bool,
    /// Whether to move special filesystems.
    pub move_special: bool,
}

/// Switch-root method.
#[derive(Clone, Debug, Default)]
pub enum SwitchRootMethod {
    /// Use `pivot_root` system call.
    #[default]
    PivotRoot,
    /// Use chroot system call.
    Chroot,
}

/// Pre-switch phase configuration.
#[derive(Clone, Debug, Default)]
pub struct PreSwitchPhase {
    /// Kernel module loading configuration.
    pub modules: Option<ModulesConfig>,
    /// Network configuration (unsupported yet).
    pub network: Option<NetworkConfig>,
    /// Mount specifications.
    pub mounts: Vec<MountSpec>,
    /// Hook specifications.
    pub hooks: Vec<HookSpec>,
    /// Environment variables to set.
    pub environment: Vec<(String, String)>,
    /// Symlinks to create.
    pub symlinks: Vec<Symlink>,
    /// Sysctl configuration (unsupported yet).
    pub sysctl: Sysctl,
    /// Tmpfiles configuration (unsupported yet).
    pub tmpfiles: Option<Tmpfile>,
    /// Security configuration (unsupported yet).
    pub security: Option<Security>,
    /// Clock configuration (unsupported yet).
    pub clock: Option<Clock>,
}

/// Post-switch phase configuration.
#[derive(Clone, Debug)]
pub struct PostSwitchPhase {
    /// Kernel module loading configuration.
    pub modules: Option<ModulesConfig>,
    /// Network configuration (unsupported yet).
    pub network: Option<NetworkConfig>,
    /// Mount specifications.
    pub mounts: Vec<MountSpec>,
    /// Hook specifications.
    pub hooks: Vec<HookSpec>,
    /// Environment variables to set.
    pub environment: Vec<(String, String)>,
    /// Symlinks to create.
    pub symlinks: Vec<Symlink>,
    /// Sysctl configuration (unsupported yet).
    pub sysctl: Sysctl,
    /// Tmpfiles configuration (unsupported yet).
    pub tmpfiles: Option<Tmpfile>,
    /// Security configuration (unsupported yet).
    pub security: Option<Security>,
    /// Clock configuration (unsupported yet).
    pub clock: Option<Clock>,
    /// Handoff configuration.
    pub handoff: Handoff,
    /// Shutdown configuration.
    pub shutdown: Option<Shutdown>,
}

/// Mount specification.
#[derive(Clone, Debug)]
pub struct MountSpec {
    pub source: MountSource,
    /// Mount target path.
    pub target: std::path::PathBuf,
    pub fstype: String,
    pub flags: nix::mount::MsFlags,
    pub options: Option<String>,
    pub overlay: bool,
    pub on_failure: MountFailurePolicy,
}

impl Default for MountSpec {
    fn default() -> Self {
        Self {
            source: MountSource::Virtual,
            target: std::path::PathBuf::new(),
            fstype: String::new(),
            flags: nix::mount::MsFlags::empty(),
            options: None,
            overlay: false,
            on_failure: MountFailurePolicy::Warn,
        }
    }
}

/// Mount source.
#[derive(Clone, Debug)]
pub enum MountSource {
    Device(String),
    VirtiofsTag(String),
    NinePTag(String),
    Virtual,
    Nfs { server: String, export: String },
    Overlay(OverlaySpec),
}

/// Mount failure policy.
#[derive(Clone, Debug, Default)]
pub enum MountFailurePolicy {
    #[default]
    Warn,
    Abort,
    Retry,
}

/// Overlay filesystem specification.
#[derive(Clone, Debug)]
pub struct OverlaySpec {
    pub lower: String,
    pub upper: String,
    pub work: String,
}

/// Hook specification.
#[derive(Clone, Debug)]
pub struct HookSpec {
    pub name: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub timeout: Duration,
    pub on_failure: MountFailurePolicy,
}

/// Network configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub interfaces: Vec<InterfaceConfig>,
}

/// Network mode.
#[derive(Clone, Debug, Default)]
pub enum NetworkMode {
    #[default]
    Dhcp,
    Static,
}

/// Network interface configuration.
#[derive(Clone, Debug, Default)]
pub struct InterfaceConfig {
    pub name: String,
    pub address: Option<String>,
    pub netmask: Option<String>,
    pub gateway: Option<String>,
}

/// Sysctl configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Sysctl {
    pub key_values: HashMap<String, String>,
}

impl From<HashMap<String, String>> for Sysctl {
    fn from(key_values: HashMap<String, String>) -> Self { Self { key_values } }
}

/// Tmpfile configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Tmpfile {
    /// Path to create.
    pub path: std::path::PathBuf,
    pub mode: String,
    pub r#type: String,
}

/// Security configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Security {
    pub selinux: Option<Selinux>,
    pub apparmor: Option<Apparmor>,
}

/// `SELinux` configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Selinux {
    pub enabled: bool,
    pub policy: Option<String>,
}

/// `AppArmor` configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Apparmor {
    pub enabled: bool,
    pub profile: Option<String>,
}

/// Clock configuration (unsupported yet).
#[derive(Clone, Debug, Default)]
pub struct Clock {
    pub rtc_sync: bool,
}

/// Symlink specification.
#[derive(Clone, Debug, Default)]
pub struct Symlink {
    /// Source path (target of symlink).
    pub source: std::path::PathBuf,
    /// Destination path (where symlink is created).
    pub target: std::path::PathBuf,
}

/// Handoff configuration.
#[derive(Clone, Debug)]
pub struct Handoff {
    pub mode: HandoffMode,
    pub boot_script: Option<BootScriptConfig>,
}

/// Handoff mode.
#[derive(Clone, Debug)]
pub enum HandoffMode {
    Supervise(ocelot_supervise::OrchestratorConfig),
    Shell(ShellConfig),
}

/// Shutdown configuration.
#[derive(Clone, Debug, Default)]
pub struct Shutdown {
    pub timeout: Duration,
    pub sync: bool,
    pub umount_all: bool,
}

/// Configuration for an extra virtiofs mount.
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
    use super::{ModulesConfig, OnFailureConfig, OnFailurePolicy, RootConfig};

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
            device: std::path::PathBuf::from("/dev/vda2"),
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
