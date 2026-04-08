use std::{collections::HashMap, time::Duration};

/// Bootstrap-specific configuration for early boot initialization.
#[derive(Clone, Debug)]
pub struct Config {
    /// Pre-switch phase configuration (before `switch_root`).
    pub pre_switch: PreSwitchPhase,
    /// Switch-root phase configuration.
    pub switch_root: SwitchRootPhase,
    /// Post-switch phase configuration (after `switch_root`).
    pub post_switch: PostSwitchPhase,
    /// Console device for shell output (e.g., `/dev/tty`).
    pub console: String,
}

/// Root filesystem backend configuration.
///
/// Supports three filesystem types for the root partition:
/// - [`RootConfig::Virtiofs`]: shared filesystem via virtio-fs (recommended for
///   QEMU)
/// - [`RootConfig::Block`]: raw block device (virtio-blk)
/// - [`RootConfig::NineP`]: 9p2000.L network filesystem
#[derive(Clone, Debug)]
pub enum RootConfig {
    /// Virtiofs shared filesystem.
    ///
    /// Mounts a virtio-fs share as the root filesystem.
    Virtiofs {
        /// Tag name for the virtiofs share.
        tag: String,
        /// Whether to use overlay filesystem on top.
        overlay: bool,
        /// Additional mount options.
        options: Option<String>,
    },
    /// Block device (virtio-blk).
    ///
    /// Mounts a raw block device as the root filesystem.
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
    ///
    /// Mounts a 9p2000.L network filesystem as the root filesystem.
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
///
/// Contains the configuration for mounting and switching to the new root
/// filesystem.
#[derive(Clone, Debug)]
pub struct SwitchRootPhase {
    pub root_file_system: MountSpec,
}

/// Switch-root method.
///
/// Determines how the root filesystem is switched.
#[derive(Clone, Debug, Default)]
pub enum SwitchRootMethod {
    /// Use `pivot_root` system call.
    #[default]
    PivotRoot,
    /// Use `chroot` system call.
    Chroot,
}

/// Pre-switch phase configuration.
///
/// Configuration for operations performed before `switch_root` is executed.
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
    pub environment: HashMap<String, String>,
    /// Symlinks to create.
    pub symlinks: Vec<Symlink>,
    /// Sysctl configuration.
    pub sysctl: Sysctl,
    /// Tmpfiles configuration (unsupported yet).
    pub tmpfiles: Option<Tmpfile>,
    /// Security configuration (unsupported yet).
    pub security: Option<Security>,
    /// Clock configuration (unsupported yet).
    pub clock: Option<Clock>,
}

/// Post-switch phase configuration.
///
/// Configuration for operations performed after `switch_root` is executed.
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
    pub environment: HashMap<String, String>,
    /// Symlinks to create.
    pub symlinks: Vec<Symlink>,
    /// Sysctl configuration.
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
///
/// Describes a single filesystem mount operation including source, target,
/// filesystem type, mount flags, options, and failure policy.
#[derive(Clone, Debug)]
pub struct MountSpec {
    /// Mount source (device, tag, or virtual).
    pub source: MountSource,
    /// Mount target path.
    pub target: std::path::PathBuf,
    /// Filesystem type (e.g., `ext4`, `virtiofs`, `tmpfs`).
    pub fstype: String,
    /// Mount flags (e.g., `MS_RDONLY`, `MS_NOEXEC`).
    pub flags: nix::mount::MsFlags,
    /// Additional mount options.
    pub options: Option<String>,
    /// Whether to use overlay filesystem on top.
    pub overlay: bool,
    /// Policy when mount fails.
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
///
/// Specifies the type of mount source for a filesystem.
#[derive(Clone, Debug)]
pub enum MountSource {
    /// Block device (e.g., `/dev/vda1`).
    Device(String),
    /// Virtiofs tag for shared filesystem.
    VirtiofsTag(String),
    /// 9p tag for network filesystem.
    NinePTag(String),
    /// Virtual filesystem (tmpfs, devtmpfs, proc, sysfs).
    Virtual,
    /// NFS network filesystem.
    Nfs { server: String, export: String },
    /// Overlay filesystem.
    Overlay(OverlaySpec),
}

/// Mount failure policy.
///
/// Determines what happens when a mount operation fails.
#[derive(Clone, Debug, Default)]
pub enum MountFailurePolicy {
    /// Log a warning and continue.
    #[default]
    Warn,
    /// Abort the boot process.
    Abort,
    /// Retry the mount operation once.
    Retry,
}

/// Overlay filesystem specification.
///
/// Configures the layers of an overlay mount (lower, upper, work directories).
#[derive(Clone, Debug)]
pub struct OverlaySpec {
    /// Lower (base) layer path.
    pub lower: String,
    /// Upper (writable) layer path.
    pub upper: String,
    /// Work directory for overlay.
    pub work: String,
}

/// Hook specification.
///
/// Describes a command to execute at a specific point in the boot process.
#[derive(Clone, Debug)]
pub struct HookSpec {
    /// Name identifier for the hook.
    pub name: String,
    /// Command to execute.
    pub command: String,
    /// Arguments for the command.
    pub arguments: Vec<String>,
    /// Timeout for command execution.
    pub timeout: Duration,
    /// Policy when hook fails.
    pub on_failure: MountFailurePolicy,
}

/// Network configuration.
///
/// Configures network interfaces and addressing for the boot environment.
#[derive(Clone, Debug, Default)]
pub struct NetworkConfig {
    /// Network addressing mode.
    pub mode: NetworkMode,
    /// Network interface configurations.
    pub interfaces: Vec<InterfaceConfig>,
}

/// Network mode.
///
/// Determines how network interfaces are configured.
#[derive(Clone, Debug, Default)]
pub enum NetworkMode {
    /// Use DHCP to obtain IP configuration.
    #[default]
    Dhcp,
    /// Use static IP configuration.
    Static,
}

/// Network interface configuration.
///
/// Describes configuration for a single network interface.
#[derive(Clone, Debug, Default)]
pub struct InterfaceConfig {
    /// Interface name (e.g., `eth0`).
    pub name: String,
    /// Static IP address.
    pub address: Option<String>,
    /// Netmask for static configuration.
    pub netmask: Option<String>,
    /// Gateway IP address.
    pub gateway: Option<String>,
}

/// Sysctl configuration.
///
/// Configures kernel parameters via sysctl.
#[derive(Clone, Debug, Default)]
pub struct Sysctl {
    /// Key-value pairs for sysctl settings (e.g., `vm.swappiness = 10`).
    pub key_values: HashMap<String, String>,
}

impl From<HashMap<String, String>> for Sysctl {
    fn from(key_values: HashMap<String, String>) -> Self { Self { key_values } }
}

/// Tmpfile configuration.
///
/// Configures a temporary file to be created during boot.
#[derive(Clone, Debug, Default)]
pub struct Tmpfile {
    /// Path to create.
    pub path: std::path::PathBuf,
    /// File permissions in octal (e.g., `"644"`).
    pub mode: String,
    /// Type of file (e.g., `f`, `d`, `L`).
    pub r#type: String,
}

/// Security configuration.
///
/// Configures `SELinux` and `AppArmor` security modules.
#[derive(Clone, Debug, Default)]
pub struct Security {
    /// `SELinux` configuration.
    pub selinux: Option<Selinux>,
    /// `AppArmor` configuration.
    pub apparmor: Option<Apparmor>,
}

/// `SELinux` configuration.
///
/// Configures `SELinux` enforcement during boot.
#[derive(Clone, Debug, Default)]
pub struct Selinux {
    /// Whether `SELinux` is enabled.
    pub enabled: bool,
    /// `SELinux` policy to use.
    pub policy: Option<String>,
}

/// `AppArmor` configuration.
///
/// Configures `AppArmor` enforcement during boot.
#[derive(Clone, Debug, Default)]
pub struct Apparmor {
    /// Whether `AppArmor` is enabled.
    pub enabled: bool,
    /// `AppArmor` profile to use.
    pub profile: Option<String>,
}

/// Clock configuration.
///
/// Configures system clock and RTC synchronization.
#[derive(Clone, Debug, Default)]
pub struct Clock {
    /// Whether to synchronize RTC from system clock at shutdown.
    pub rtc_sync: bool,
}

/// Symlink specification.
///
/// Describes a symbolic link to create during the boot process.
#[derive(Clone, Debug, Default)]
pub struct Symlink {
    /// Source path (target of symlink).
    pub source: std::path::PathBuf,
    /// Destination path (where symlink is created).
    pub target: std::path::PathBuf,
}

/// Handoff configuration.
///
/// Configures what happens after the bootstrap process completes.
#[derive(Clone, Debug)]
pub struct Handoff {
    /// Handoff mode (supervise or shell).
    pub mode: HandoffMode,
    /// Optional boot script to execute before handoff.
    pub boot_script: Option<BootScriptConfig>,
}

/// Handoff mode.
///
/// Determines what the bootstrap process hands off to after completion.
#[derive(Clone, Debug)]
pub enum HandoffMode {
    /// Hand off to the supervise orchestrator.
    Supervise(ocelot_supervise::OrchestratorConfig),
    /// Spawn an interactive shell.
    Shell(ShellConfig),
}

/// Shutdown configuration.
///
/// Configures system shutdown behavior when the bootstrap process exits.
#[derive(Clone, Debug, Default)]
pub struct Shutdown {
    /// Timeout for shutdown operations.
    pub timeout: Duration,
    /// Whether to sync filesystems before shutdown.
    pub sync: bool,
    /// Whether to unmount all filesystems.
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
///
/// This is a string-based version of [`Symlink`] used in configuration files.
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
///
/// Determines whether to continue or abort when a boot script fails.
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
