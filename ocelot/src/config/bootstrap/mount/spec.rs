use nix::mount::MsFlags;
use serde::Deserialize;

use crate::config::bootstrap::mount::AtimeMode;

/// `MountFailurePolicy`: Mount failure policy (serialization type).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MountFailurePolicy {
    #[default]
    Warn,
    Abort,
    Retry,
}

/// `MountSpecConfig`: Mount specification config (serialization type).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, tag = "type")]
pub enum MountSpecConfig {
    Virtiofs {
        target: String,
        tag: String,
        overlay: bool,
        #[serde(default)]
        options: Option<String>,
        #[serde(flatten)]
        flags: MountFlags,
    },
    Block {
        target: String,
        device: String,
        fstype: String,
        overlay: bool,
        #[serde(default)]
        options: Option<String>,
        #[serde(flatten)]
        flags: MountFlags,
    },
    #[serde(rename = "9p")]
    NineP {
        target: String,
        tag: String,
        fstype: Option<String>,
        overlay: bool,
        #[serde(default)]
        options: Option<String>,
        #[serde(flatten)]
        flags: MountFlags,
    },
    Virtual {
        target: String,
        fstype: String,
        #[serde(default)]
        options: Option<String>,
        #[serde(flatten)]
        flags: MountFlags,
    },
    Nfs {
        target: String,
        server: String,
        export: String,
        fstype: Option<String>,
        #[serde(default)]
        options: Option<String>,
        #[serde(flatten)]
        flags: MountFlags,
    },
    Overlay {
        target: String,
        lower: String,
        upper: String,
        work: String,
        #[serde(flatten)]
        flags: MountFlags,
    },
}

impl From<MountSpecConfig> for ocelot_bootstrap::MountSpec {
    fn from(config: MountSpecConfig) -> Self {
        match config {
            MountSpecConfig::Virtiofs { target, tag, overlay, options, flags } => Self {
                source: ocelot_bootstrap::MountSource::VirtiofsTag(tag),
                target: target.into(),
                fstype: "virtiofs".to_string(),
                flags: flags.derive(),
                options,
                overlay,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
            MountSpecConfig::Block { target, device, fstype, overlay, options, flags } => Self {
                source: ocelot_bootstrap::MountSource::Device(device),
                target: target.into(),
                fstype,
                flags: flags.derive(),
                options,
                overlay,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
            MountSpecConfig::NineP { target, tag, fstype, overlay, options, flags } => Self {
                source: ocelot_bootstrap::MountSource::NinePTag(tag),
                target: target.into(),
                fstype: fstype.unwrap_or_else(|| "9p".to_string()),
                flags: flags.derive(),
                options,
                overlay,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
            MountSpecConfig::Virtual { target, fstype, options, flags } => Self {
                source: ocelot_bootstrap::MountSource::Virtual,
                target: target.into(),
                fstype,
                flags: flags.derive(),
                options,
                overlay: false,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
            MountSpecConfig::Nfs { target, server, export, fstype, options, flags } => Self {
                source: ocelot_bootstrap::MountSource::Nfs { server, export },
                target: target.into(),
                fstype: fstype.unwrap_or_else(|| "nfs".to_string()),
                flags: flags.derive(),
                options,
                overlay: false,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
            MountSpecConfig::Overlay { target, lower, upper, work, flags } => Self {
                source: ocelot_bootstrap::MountSource::Overlay(ocelot_bootstrap::OverlaySpec {
                    lower,
                    upper,
                    work,
                }),
                target: target.into(),
                fstype: "overlay".to_string(),
                flags: flags.derive(),
                options: None,
                overlay: false,
                on_failure: ocelot_bootstrap::MountFailurePolicy::Warn,
            },
        }
    }
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "Mount flags naturally have many independent boolean switches; this is the \
              user-facing abstraction"
)]
/// Builder for constructing `MsFlags` from boolean mount options.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MountFlags {
    read_only: bool,
    no_exec: bool,
    no_suid: bool,
    no_dev: bool,
    sync: bool,
    dir_sync: bool,
    mandatory_locks: bool,
    posix_acl: bool,
    atime: AtimeMode,
}

impl MountFlags {
    #[must_use]
    fn derive(self) -> MsFlags {
        let mut flags = MsFlags::empty();
        if self.read_only {
            flags |= MsFlags::MS_RDONLY;
        }
        if self.no_exec {
            flags |= MsFlags::MS_NOEXEC;
        }
        if self.no_suid {
            flags |= MsFlags::MS_NOSUID;
        }
        if self.no_dev {
            flags |= MsFlags::MS_NODEV;
        }
        if self.sync {
            flags |= MsFlags::MS_SYNCHRONOUS;
        }
        if self.dir_sync {
            flags |= MsFlags::MS_DIRSYNC;
        }
        if self.mandatory_locks {
            flags |= MsFlags::MS_MANDLOCK;
        }
        if self.posix_acl {
            flags |= MsFlags::MS_POSIXACL;
        }
        match self.atime {
            AtimeMode::Default => {}
            AtimeMode::NoAtime => flags |= MsFlags::MS_NOATIME,
            AtimeMode::RelAtime => flags |= MsFlags::MS_RELATIME,
            AtimeMode::StrictAtime => flags |= MsFlags::MS_STRICTATIME,
            AtimeMode::LazyTime => flags |= MsFlags::MS_LAZYTIME,
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use nix::mount::MsFlags;

    use super::*;

    #[test]
    fn test_mount_flags_builder() {
        // Test all flags set
        let builder = MountFlags {
            read_only: true,
            no_exec: true,
            no_suid: true,
            no_dev: true,
            sync: true,
            dir_sync: true,
            mandatory_locks: true,
            posix_acl: true,
            atime: AtimeMode::NoAtime,
        };
        let flags = builder.derive();
        let expected = MsFlags::MS_RDONLY
            | MsFlags::MS_NOEXEC
            | MsFlags::MS_NOSUID
            | MsFlags::MS_NODEV
            | MsFlags::MS_SYNCHRONOUS
            | MsFlags::MS_DIRSYNC
            | MsFlags::MS_MANDLOCK
            | MsFlags::MS_POSIXACL
            | MsFlags::MS_NOATIME;
        assert_eq!(flags, expected);

        // Test no flags (all false, atime Default)
        let builder = MountFlags {
            read_only: false,
            no_exec: false,
            no_suid: false,
            no_dev: false,
            sync: false,
            dir_sync: false,
            mandatory_locks: false,
            posix_acl: false,
            atime: AtimeMode::Default,
        };
        let flags = builder.derive();
        assert_eq!(flags, MsFlags::empty());

        // Test each atime variant individually with no other flags
        for (atime, flag) in [
            (AtimeMode::NoAtime, MsFlags::MS_NOATIME),
            (AtimeMode::RelAtime, MsFlags::MS_RELATIME),
            (AtimeMode::StrictAtime, MsFlags::MS_STRICTATIME),
            (AtimeMode::LazyTime, MsFlags::MS_LAZYTIME),
        ] {
            let builder = MountFlags {
                read_only: false,
                no_exec: false,
                no_suid: false,
                no_dev: false,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: false,
                atime,
            };
            let flags = builder.derive();
            assert_eq!(flags, flag);
        }

        // Test partial flags: read-only + no-suid + relatime
        let builder = MountFlags {
            read_only: true,
            no_exec: false,
            no_suid: true,
            no_dev: false,
            sync: false,
            dir_sync: false,
            mandatory_locks: false,
            posix_acl: false,
            atime: AtimeMode::RelAtime,
        };
        let flags = builder.derive();
        let expected = MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_RELATIME;
        assert_eq!(flags, expected);
    }

    #[test]
    fn test_atime_mode_deserialization() {
        let yaml = r"
type: virtual
target: /mnt
fstype: ext4
atime: noAtime
";
        let config: MountSpecConfig = serde_yaml::from_str(yaml).unwrap();
        match config {
            MountSpecConfig::Virtual { flags, .. } => {
                assert!(matches!(flags.atime, AtimeMode::NoAtime));
                assert!(!flags.read_only);
            }
            _ => panic!("wrong variant"),
        }

        let yaml = r"
type: virtual
target: /mnt
fstype: ext4
atime: strictAtime
";
        let config: MountSpecConfig = serde_yaml::from_str(yaml).unwrap();
        match config {
            MountSpecConfig::Virtual { flags, .. } => {
                assert!(matches!(flags.atime, AtimeMode::StrictAtime));
                assert!(!flags.read_only);
            }
            _ => panic!("wrong variant"),
        }

        let yaml = r"
type: virtual
target: /mnt
fstype: ext4
atime: lazyTime
";
        let config: MountSpecConfig = serde_yaml::from_str(yaml).unwrap();
        match config {
            MountSpecConfig::Virtual { flags, .. } => {
                assert!(matches!(flags.atime, AtimeMode::LazyTime));
                assert!(!flags.read_only);
            }
            _ => panic!("wrong variant"),
        }

        // Default when atime not specified
        let yaml = r"
type: virtual
target: /mnt
fstype: ext4
";
        let config: MountSpecConfig = serde_yaml::from_str(yaml).unwrap();
        match config {
            MountSpecConfig::Virtual { flags, .. } => {
                assert!(matches!(flags.atime, AtimeMode::Default));
                assert!(!flags.read_only);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_boolean_flags_to_msflags() {
        // Test all flags set
        let config = MountSpecConfig::Virtual {
            target: "/mnt".to_string(),
            fstype: "ext4".to_string(),
            options: None,
            flags: MountFlags {
                read_only: true,
                no_exec: true,
                no_suid: true,
                no_dev: true,
                sync: true,
                dir_sync: true,
                mandatory_locks: true,
                posix_acl: true,
                atime: AtimeMode::NoAtime,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        let expected = MsFlags::MS_RDONLY
            | MsFlags::MS_NOEXEC
            | MsFlags::MS_NOSUID
            | MsFlags::MS_NODEV
            | MsFlags::MS_SYNCHRONOUS
            | MsFlags::MS_DIRSYNC
            | MsFlags::MS_MANDLOCK
            | MsFlags::MS_POSIXACL
            | MsFlags::MS_NOATIME;
        assert_eq!(mount_spec.flags, expected);

        // Test no flags (default)
        let config = MountSpecConfig::Virtual {
            target: "/mnt".to_string(),
            fstype: "ext4".to_string(),
            options: None,
            flags: MountFlags {
                read_only: false,
                no_exec: false,
                no_suid: false,
                no_dev: false,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: false,
                atime: AtimeMode::Default,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        assert_eq!(mount_spec.flags, MsFlags::empty());

        // Test atime variants
        for (atime, flag) in [
            (AtimeMode::NoAtime, MsFlags::MS_NOATIME),
            (AtimeMode::RelAtime, MsFlags::MS_RELATIME),
            (AtimeMode::StrictAtime, MsFlags::MS_STRICTATIME),
            (AtimeMode::LazyTime, MsFlags::MS_LAZYTIME),
        ] {
            let config = MountSpecConfig::Virtual {
                target: "/mnt".to_string(),
                fstype: "ext4".to_string(),
                options: None,
                flags: MountFlags {
                    read_only: false,
                    no_exec: false,
                    no_suid: false,
                    no_dev: false,
                    sync: false,
                    dir_sync: false,
                    mandatory_locks: false,
                    posix_acl: false,
                    atime,
                },
            };
            let mount_spec: ocelot_bootstrap::MountSpec = config.into();
            assert_eq!(mount_spec.flags, flag);
        }
    }

    #[test]
    fn test_virtiofs_flags_conversion() {
        let config = MountSpecConfig::Virtiofs {
            target: "/mnt".to_string(),
            tag: "rootfs".to_string(),
            overlay: true,
            options: Some("rw".to_string()),
            flags: MountFlags {
                read_only: true,
                no_exec: false,
                no_suid: true,
                no_dev: false,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: true,
                atime: AtimeMode::RelAtime,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        assert!(
            matches!(mount_spec.source, ocelot_bootstrap::MountSource::VirtiofsTag(tag) if tag == "rootfs")
        );
        assert_eq!(mount_spec.target, std::path::PathBuf::from("/mnt"));
        assert_eq!(mount_spec.fstype, "virtiofs");
        assert!(mount_spec.overlay);
        assert_eq!(mount_spec.options, Some("rw".to_string()));
        let expected =
            MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_POSIXACL | MsFlags::MS_RELATIME;
        assert_eq!(mount_spec.flags, expected);
        assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
    }

    #[test]
    fn test_block_flags_conversion() {
        let config = MountSpecConfig::Block {
            target: "/boot".to_string(),
            device: "/dev/vda1".to_string(),
            fstype: "vfat".to_string(),
            overlay: false,
            options: None,
            flags: MountFlags {
                read_only: true,
                no_exec: false,
                no_suid: false,
                no_dev: true,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: false,
                atime: AtimeMode::Default,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        assert!(
            matches!(mount_spec.source, ocelot_bootstrap::MountSource::Device(dev) if dev == "/dev/vda1")
        );
        assert_eq!(mount_spec.fstype, "vfat");
        let expected = MsFlags::MS_RDONLY | MsFlags::MS_NODEV;
        assert_eq!(mount_spec.flags, expected);
    }

    #[test]
    fn test_ninep_flags_conversion() {
        let config = MountSpecConfig::NineP {
            target: "/dev".to_string(),
            tag: "dev".to_string(),
            fstype: None,
            overlay: false,
            options: None,
            flags: MountFlags {
                read_only: false,
                no_exec: false,
                no_suid: false,
                no_dev: false,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: false,
                atime: AtimeMode::StrictAtime,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        assert!(
            matches!(mount_spec.source, ocelot_bootstrap::MountSource::NinePTag(tag) if tag == "dev")
        );
        assert_eq!(mount_spec.fstype, "9p");
        assert_eq!(mount_spec.flags, MsFlags::MS_STRICTATIME);
    }

    #[test]
    fn test_overlay_flags_conversion() {
        let config = MountSpecConfig::Overlay {
            target: "/overlay".to_string(),
            lower: "/lower".to_string(),
            upper: "/upper".to_string(),
            work: "/work".to_string(),
            flags: MountFlags {
                read_only: false,
                no_exec: false,
                no_suid: false,
                no_dev: false,
                sync: false,
                dir_sync: false,
                mandatory_locks: false,
                posix_acl: false,
                atime: AtimeMode::LazyTime,
            },
        };
        let mount_spec: ocelot_bootstrap::MountSpec = config.into();
        assert!(matches!(mount_spec.source, ocelot_bootstrap::MountSource::Overlay(_)));
        assert_eq!(mount_spec.fstype, "overlay");
        assert_eq!(mount_spec.flags, MsFlags::MS_LAZYTIME);
        assert_eq!(mount_spec.options, None);
    }
}
