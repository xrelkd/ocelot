use super::{ModulesConfig, OnFailureConfig, OnFailurePolicy, RootConfig};

#[test]
fn test_root_config_virtiofs_overlay() {
    let config = RootConfig::Virtiofs { tag: "rootfs".to_string(), overlay: true, options: None };
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
    assert_eq!(config.directory, std::path::PathBuf::from("/lib/modules"));
    assert!(config.module_file_names.is_empty());
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
