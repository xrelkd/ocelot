## ADDED Requirements

### Requirement: Bootstrap configuration SHALL be split into independent files in ocelot/src/config/

Configuration types SHALL be organized into separate files in `ocelot/src/config/bootstrap/` directory instead of a single monolithic file.

#### Scenario: New directory structure exists

- **WHEN** listing `ocelot/src/config/bootstrap/`
- **THEN** the directory contains separate files for: bootstrap.rs, pre_switch.rs, switch_root.rs, post_switch.rs, mount_spec.rs, modules_config.rs, network_config.rs, hook_spec.rs, sysctl_config.rs, tmpfile_config.rs, security_config.rs, clock_config.rs, handoff_config.rs, shutdown_config.rs

#### Scenario: Main bootstrap.rs exports all submodules

- **WHEN** importing `ocelot::config::bootstrap`
- **THEN** all submodules are accessible through re-exports

### Requirement: Deprecated configuration structs and enums SHALL be removed entirely

All deprecated configuration structures from the previous flat BootstrapConfig SHALL be deleted from the codebase, not merely marked with `#[expect(dead_code)]`.

#### Scenario: No deprecated structs remain

- **WHEN** searching the codebase for deprecated struct names (e.g., old flat BootstrapConfig fields)
- **THEN** no instances are found in source files

#### Scenario: No deprecated enums remain

- **WHEN** searching the codebase for deprecated enum variants
- **THEN** no instances are found in source files

### Requirement: Configuration files SHALL follow a consistent structure

Each configuration file SHALL contain exactly one primary configuration type with its related sub-types, following the pattern: `<name>_config.rs`.

#### Scenario: Mount spec file structure

- **WHEN** examining `mount_spec.rs`
- **THEN** it contains `MountSpecConfig` serialization type and `MountSpec` runtime type with their `From` implementation

#### Scenario: Modules config file structure

- **WHEN** examining `modules_config.rs`
- **THEN** it contains `ModulesConfig` serialization type and `ModulesConfig` runtime type with their `From` implementation

### Requirement: Main BootstrapConfig SHALL aggregate phase configs

The main `BootstrapConfig` in `bootstrap.rs` SHALL contain `pre_switch: PreSwitchConfig`, `switch_root: SwitchRootConfig`, and `post_switch: PostSwitchConfig` fields.

#### Scenario: BootstrapConfig has three phases

- **WHEN** examining the main BootstrapConfig struct
- **THEN** it has exactly three top-level fields corresponding to the three phases

#### Scenario: BootstrapConfig has no legacy flat fields

- **WHEN** examining the main BootstrapConfig struct
- **THEN** it does not contain any of the legacy flat fields (modules, extra_virtiofs_mounts, symlinks, boot_script, etc.)
