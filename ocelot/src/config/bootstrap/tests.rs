//! Unit tests for bootstrap configuration module.

use tempfile::tempdir;

use super::*;

// ============================================================================
// BootstrapConfig Tests
// ============================================================================

#[test]
fn test_bootstrap_config_default() {
    let config = BootstrapConfig::default();
    assert_eq!(config.console, "console");
    assert_eq!(config.log_level, tracing::Level::INFO);
    assert!(config.pre_switch.modules.is_none());
    assert!(config.pre_switch.network.is_none());
    assert!(config.pre_switch.mounts.is_empty());
    assert!(config.pre_switch.hooks.is_empty());
    assert!(config.pre_switch.environment.is_empty());
    assert!(config.pre_switch.symlinks.is_empty());
    assert!(config.pre_switch.modules.is_none());
    assert!(config.pre_switch.security.is_none());
    assert!(config.pre_switch.clock.is_none());
    // Check switch_root defaults
    assert!(!config.switch_root.cleanup_old_root);
    assert!(!config.switch_root.move_special);
    assert!(config.post_switch.modules.is_none());
    assert!(config.post_switch.network.is_none());
    assert!(config.post_switch.mounts.is_empty());
    assert!(config.post_switch.hooks.is_empty());
    assert!(config.post_switch.symlinks.is_empty());
    assert!(config.post_switch.modules.is_none());
    assert!(config.post_switch.security.is_none());
    assert!(config.post_switch.clock.is_none());
    // Handoff defaults to Supervise mode with no config
    assert!(matches!(config.post_switch.handoff.mode, HandoffMode::Supervise));
    assert!(config.post_switch.handoff.supervise.is_none());
    assert!(config.post_switch.handoff.shell.is_none());
    assert!(config.post_switch.shutdown.is_none());
}

#[test]
fn test_bootstrap_config_deserialize_minimal() {
    let yaml = b"
console: myconsole
logLevel: debug
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
    - [PATH, /usr/bin]
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
    - [HOME, /root]
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
}

#[test]
fn test_bootstrap_config_validate_handoff_neither_shell_nor_supervise() {
    let yaml = b"
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
fn test_bootstrap_config_handoff_mode() {
    let mut config = BootstrapConfig::default();
    assert!(matches!(config.handoff_mode(), HandoffMode::Supervise));

    // Change to shell via handoff config
    config.post_switch.handoff.mode = HandoffMode::Shell;
    assert!(matches!(config.handoff_mode(), HandoffMode::Shell));
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
fn test_bootstrap_config_to_bootstrap_config() {
    // Construct a valid config with supervise handoff
    let mut config = BootstrapConfig::default();
    config.post_switch.handoff.mode = HandoffMode::Supervise;
    config.post_switch.handoff.supervise =
        Some(BootstrapSuperviseConfig { processes: HashMap::default(), shutdown_timeout_secs: 30 });
    let bootstrap_config = config.to_bootstrap_config();
    assert_eq!(bootstrap_config.console, "console");
}

#[test]
fn test_bootstrap_config_load() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bootstrap.yaml");

    let yaml = b"
console: testconsole
logLevel: error
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
postSwitch:
  handoff:
    mode: shell
";
    let config: BootstrapConfig = serde_yaml::from_slice(yaml).unwrap();
    assert!(matches!(config.handoff_mode(), HandoffMode::Shell));

    let yaml2 = b"
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
fn test_validate_switch_root_methods() {
    // Test pivotRoot (default)
    let yaml_default = b"switchRoot:
  method: pivotRoot
";
    let config: BootstrapConfig = serde_yaml::from_slice(yaml_default).unwrap();
    assert_eq!(format!("{:?}", config.switch_root.method), "PivotRoot");

    // Test chroot
    let yaml_chroot = b"switchRoot:
  method: chroot
";
    let config2: BootstrapConfig = serde_yaml::from_slice(yaml_chroot).unwrap();
    assert_eq!(format!("{:?}", config2.switch_root.method), "Chroot");
}
