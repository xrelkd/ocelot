//! Unit tests for bootstrap configuration module.

use nix::mount::MsFlags;
use ocelot_bootstrap::MountSpec;
use tempfile::tempdir;

use super::{BootstrapConfig, HandoffMode};
use crate::config::bootstrap::mount::MountSpecConfig;

// ============================================================================
// BootstrapConfig Tests
// ============================================================================

#[test]
fn test_bootstrap_config_deserialize_minimal() {
    let yaml = b"
console: myconsole
logLevel: debug
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
";
    let config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    assert_eq!(config.console, "myconsole");
    assert_eq!(config.log_level, tracing::Level::DEBUG);
}

#[test]
fn test_bootstrap_config_deserialize_full() {
    let yaml = b"
console: ttyS0
logLevel: warn
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
preSwitch:
  modules:
    mode: list
    names:
      - systemd
  network:
    mode: dhcp
  mounts:
    - type: virtiofs
      tag: rootfs
      target: /newroot
      overlay: false
  environment:
    PATH: /usr/bin
  hooks:
    - name: pre-hook
      command: /bin/pre-hook
      arguments: []
postSwitch:
  modules:
    mode: list
    names:
      - networkd

  environment:
    HOME: /root
  handoff:
    mode: shell
    shell:
      program: /bin/bash
      arguments:
        - -l
";
    let config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    assert_eq!(config.console, "ttyS0");
    assert_eq!(config.log_level, tracing::Level::WARN);
    assert!(config.pre_switch.modules.is_some());
    assert!(config.pre_switch.network.is_some());
    assert_eq!(config.pre_switch.mounts.len(), 1);
    assert_eq!(config.pre_switch.environment.len(), 1);
    assert_eq!(config.pre_switch.hooks.len(), 1);
    assert!(config.post_switch.modules.is_some());
    assert!(config.post_switch.mounts.is_empty());
    assert_eq!(config.post_switch.environment.len(), 1);
    assert!(matches!(config.post_switch.handoff.mode, HandoffMode::Shell));
    assert!(config.post_switch.handoff.shell.is_some());
    assert!(config.post_switch.handoff.supervise.is_none());
    assert!(config.post_switch.shutdown.is_none());
}

#[test]
fn test_bootstrap_config_validate_handoff_neither_shell_nor_supervise() {
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
";
    let mut config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_bootstrap_config_validate_handoff_both_shell_and_supervise_fails() {
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
    shell:
      program: /bin/sh
    supervise:
      processes: {}
";
    let mut config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_bootstrap_config_template_shell() {
    let template = BootstrapConfig::template_shell();
    assert!(!template.is_empty());

    // Deserialize and validate
    let config: BootstrapConfig = serde_yaml::from_slice(&template).unwrap();
    assert_eq!(config.console, "console");
    assert_eq!(config.log_level, tracing::Level::INFO);
    assert!(!config.pre_switch.mounts.is_empty());
    assert!(matches!(config.post_switch.handoff.mode, HandoffMode::Shell));
    assert!(config.post_switch.handoff.shell.is_some());
    assert!(config.post_switch.handoff.supervise.is_none());

    // Validate the parsed config
    let mut validated = config;
    assert!(validated.validate().is_ok());
}

#[test]
fn test_bootstrap_config_template_supervise() {
    let template = BootstrapConfig::template_supervise();
    assert!(!template.is_empty());

    // Deserialize and validate
    let config: BootstrapConfig = serde_yaml::from_slice(&template).unwrap();
    assert_eq!(config.console, "console");
    assert_eq!(config.log_level, tracing::Level::INFO);
    assert!(!config.pre_switch.mounts.is_empty());
    assert!(matches!(config.post_switch.handoff.mode, HandoffMode::Supervise));
    assert!(config.post_switch.handoff.supervise.is_some());
    assert!(config.post_switch.handoff.shell.is_none());

    // Validate the parsed config
    let mut validated = config;
    assert!(validated.validate().is_ok());
}

#[test]
fn test_bootstrap_config_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bootstrap.yaml");

    let yaml = b"
console: testconsole
logLevel: error
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
preSwitch:
  mounts:
    - type: virtiofs
      tag: rootfs
      target: /newroot
      overlay: false
postSwitch:
  handoff:
    mode: shell
    shell:
      program: /bin/sh
";
    std::fs::write(&path, yaml).unwrap();

    let config = BootstrapConfig::load(&path).unwrap();
    assert_eq!(config.console, "testconsole");
    assert_eq!(config.log_level, tracing::Level::ERROR);
    assert_eq!(config.pre_switch.mounts.len(), 1);
    assert!(matches!(config.post_switch.handoff.mode, HandoffMode::Shell));
}

#[test]
fn test_bootstrap_config_load_invalid_yaml() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid.yaml");

    std::fs::write(&path, b"invalid: yaml: content: {{}").unwrap();

    let result = BootstrapConfig::load(&path);
    assert!(result.is_err());
}

// ============================================================================
// Conversion and Integration Tests
// ============================================================================

#[test]
fn test_handoff_mode_conversion_in_bootstrap_config() {
    // Test that HandoffMode is correctly derived from config
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: shell
";
    let config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    assert!(matches!(config.handoff_mode(), HandoffMode::Shell));

    let yaml2 = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
";
    let config2: BootstrapConfig = serde_yaml::from_slice(yaml2).unwrap();
    assert!(matches!(config2.handoff_mode(), HandoffMode::Supervise));
}

#[test]
fn test_validate_supervise_with_valid_processes() {
    // This tests that a valid supervise configuration inside BootstrapConfig
    // validates correctly
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
    supervise:
      processes:
        init:
          program: /sbin/init
        server:
          program: /usr/bin/server
          dependsOn:
            init:
              condition: Started
";
    let mut config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_supervise_with_missing_dependency_fails() {
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
    supervise:
      processes:
        app:
          program: /usr/bin/app
          dependsOn:
            missing:
              condition: Started
";
    let mut config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_validate_supervise_with_cycle_fails() {
    let yaml = b"
switchRoot:
  rootFileSystem:
    type: virtiofs
    tag: rootfs
    target: /newroot
    overlay: false
postSwitch:
  handoff:
    mode: supervise
    supervise:
      processes:
        a:
          program: /usr/bin/a
          dependsOn:
            b:
              condition: Started
        b:
          program: /usr/bin/b
          dependsOn:
            a:
              condition: Started
";
    let mut config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_mount_spec_config_virtiofs_all_flags() {
    let yaml = b"
type: virtiofs
target: /mnt/virtiofs
tag: rootfs
overlay: false
options: cache=always
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: noAtime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(
        mount_spec.source,
        ocelot_bootstrap::MountSource::VirtiofsTag(tag)
        if tag == "rootfs"
    ));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/mnt/virtiofs"));
    assert_eq!(mount_spec.fstype, "virtiofs");
    assert!(!mount_spec.overlay);
    assert_eq!(mount_spec.options, Some("cache=always".to_string()));
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
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}

#[test]
fn test_mount_spec_config_block_all_flags() {
    let yaml = b"
type: block
target: /boot
device: /dev/vda1
fstype: vfat
overlay: false
options: ro
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: relAtime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(
        mount_spec.source,
        ocelot_bootstrap::MountSource::Device(dev)
        if dev == "/dev/vda1"
    ));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/boot"));
    assert_eq!(mount_spec.fstype, "vfat");
    assert!(!mount_spec.overlay);
    assert_eq!(mount_spec.options, Some("ro".to_string()));
    let expected = MsFlags::MS_RDONLY
        | MsFlags::MS_NOEXEC
        | MsFlags::MS_NOSUID
        | MsFlags::MS_NODEV
        | MsFlags::MS_SYNCHRONOUS
        | MsFlags::MS_DIRSYNC
        | MsFlags::MS_MANDLOCK
        | MsFlags::MS_POSIXACL
        | MsFlags::MS_RELATIME;
    assert_eq!(mount_spec.flags, expected);
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}

#[test]
fn test_mount_spec_config_ninep_all_flags() {
    let yaml = b"
type: 9p
target: /dev
tag: dev
fstype: 9p2000
overlay: false
options: trans=virtio
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: strictAtime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(
        mount_spec.source,
        ocelot_bootstrap::MountSource::NinePTag(tag)
        if tag == "dev"
    ));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/dev"));
    assert_eq!(mount_spec.fstype, "9p2000");
    assert!(!mount_spec.overlay);
    assert_eq!(mount_spec.options, Some("trans=virtio".to_string()));
    let expected = MsFlags::MS_RDONLY
        | MsFlags::MS_NOEXEC
        | MsFlags::MS_NOSUID
        | MsFlags::MS_NODEV
        | MsFlags::MS_SYNCHRONOUS
        | MsFlags::MS_DIRSYNC
        | MsFlags::MS_MANDLOCK
        | MsFlags::MS_POSIXACL
        | MsFlags::MS_STRICTATIME;
    assert_eq!(mount_spec.flags, expected);
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}

#[test]
fn test_mount_spec_config_virtual_all_flags() {
    let yaml = b"
type: virtual
target: /mnt/virtual
fstype: tmpfs
options: size=100m
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: lazyTime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(mount_spec.source, ocelot_bootstrap::MountSource::Virtual));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/mnt/virtual"));
    assert_eq!(mount_spec.fstype, "tmpfs");
    assert_eq!(mount_spec.options, Some("size=100m".to_string()));
    let expected = MsFlags::MS_RDONLY
        | MsFlags::MS_NOEXEC
        | MsFlags::MS_NOSUID
        | MsFlags::MS_NODEV
        | MsFlags::MS_SYNCHRONOUS
        | MsFlags::MS_DIRSYNC
        | MsFlags::MS_MANDLOCK
        | MsFlags::MS_POSIXACL
        | MsFlags::MS_LAZYTIME;
    assert_eq!(mount_spec.flags, expected);
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}

#[test]
fn test_mount_spec_config_nfs_all_flags() {
    let yaml = b"
type: nfs
target: /mnt/nfs
server: 192.168.1.100
export: /export
fstype: nfs4
options: soft,timeo=100
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: noAtime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(
        mount_spec.source,
        ocelot_bootstrap::MountSource::Nfs { server, export }
        if server == "192.168.1.100" && export == "/export"
    ));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/mnt/nfs"));
    assert_eq!(mount_spec.fstype, "nfs4");
    assert_eq!(mount_spec.options, Some("soft,timeo=100".to_string()));
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
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}

#[test]
fn test_mount_spec_config_overlay_all_flags() {
    let yaml = b"
type: overlay
target: /mnt/overlay
lower: /lower
upper: /upper
work: /work
readOnly: true
noExec: true
noSuid: true
noDev: true
sync: true
dirSync: true
mandatoryLocks: true
posixAcl: true
atime: relAtime
";
    let config: MountSpecConfig = serde_yaml::from_slice(yaml).unwrap();
    let mount_spec: MountSpec = config.into();
    assert!(matches!(
        mount_spec.source,
        ocelot_bootstrap::MountSource::Overlay(ocelot_bootstrap::OverlaySpec { lower, upper, work })
        if lower == "/lower" && upper == "/upper" && work == "/work"
    ));
    assert_eq!(mount_spec.target, std::path::PathBuf::from("/mnt/overlay"));
    assert_eq!(mount_spec.fstype, "overlay");
    assert_eq!(mount_spec.options, None);
    let expected = MsFlags::MS_RDONLY
        | MsFlags::MS_NOEXEC
        | MsFlags::MS_NOSUID
        | MsFlags::MS_NODEV
        | MsFlags::MS_SYNCHRONOUS
        | MsFlags::MS_DIRSYNC
        | MsFlags::MS_MANDLOCK
        | MsFlags::MS_POSIXACL
        | MsFlags::MS_RELATIME;
    assert_eq!(mount_spec.flags, expected);
    assert!(matches!(mount_spec.on_failure, ocelot_bootstrap::MountFailurePolicy::Warn));
}
