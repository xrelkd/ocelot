## ADDED Requirements

### Requirement: BootstrapConfig SHALL have three-tier phase structure

The serialization-layer `BootstrapConfig` SHALL consist of three top-level fields: `pre_switch: PreSwitchConfig`, `switch_root: SwitchRootConfig`, and `post_switch: PostSwitchConfig`. All legacy flat fields (modules, extra_virtiofs_mounts, symlinks, boot_script, etc.) SHALL be removed from the top level.

#### Scenario: Deserialize valid three-tier config

- **WHEN** a YAML config with `preSwitch`, `switchRoot`, and `postSwitch` sections is loaded
- **THEN** `BootstrapConfig::load()` returns a valid config with all three phases populated

#### Scenario: Reject unknown fields

- **WHEN** a YAML config contains fields outside the three-tier structure
- **THEN** deserialization fails with `deny_unknown_fields` error

### Requirement: PreSwitchConfig SHALL contain all pre-switch subsystems

`PreSwitchConfig` SHALL have optional fields for: `modules`, `network`, `mounts`, `hooks`, `environment`, `symlinks`, `sysctl`, `tmpfiles`, `security`, `clock`. All fields default to empty/None.

#### Scenario: Empty preSwitch config is valid

- **WHEN** `preSwitch` section is empty or omitted
- **THEN** `PreSwitchConfig` defaults to all-empty state

#### Scenario: PreSwitchConfig with all subsystems

- **WHEN** a YAML config specifies all subsystems under `preSwitch`
- **THEN** each field is deserialized into its corresponding type

### Requirement: PostSwitchConfig SHALL contain all post-switch subsystems plus handoff and shutdown

`PostSwitchConfig` SHALL have the same subsystem fields as `PreSwitchConfig` plus `handoff: Option<HandoffConfig>` and `shutdown: Option<ShutdownConfig>`.

#### Scenario: PostSwitchConfig with handoff

- **WHEN** `postSwitch.handoff` specifies `mode: supervise` with process definitions
- **THEN** `HandoffConfig` is populated with supervise configuration

#### Scenario: PostSwitchConfig with shutdown

- **WHEN** `postSwitch.shutdown` specifies `timeout`, `sync`, and `umountAll`
- **THEN** `ShutdownConfig` is populated with shutdown parameters

### Requirement: SwitchRootConfig SHALL define root switching parameters

`SwitchRootConfig` SHALL have fields: `method` (pivot_root | chroot), `oldRootDir`, `cleanupOldRoot`, and `moveSpecial` (list of paths to move).

#### Scenario: Default switchRoot config

- **WHEN** `switchRoot` section is empty or omitted
- **THEN** defaults to `method: pivot_root`, `oldRootDir: /oldroot`, `cleanupOldRoot: true`, `moveSpecial: [proc, sys, dev, dev/pts, dev/shm, run]`

#### Scenario: Chroot fallback method

- **WHEN** `switchRoot.method` is set to `chroot`
- **THEN** the switch_root code uses chroot instead of pivot_root

### Requirement: Runtime Config SHALL mirror phase structure

The runtime `Config` in `crates/bootstrap/src/config.rs` SHALL have `pre_switch: PreSwitchPhase`, `switch_root: SwitchRootPhase`, and `post_switch: PostSwitchPhase` fields, mirroring the serialization layer.

#### Scenario: Convert BootstrapConfig to runtime Config

- **WHEN** `BootstrapConfig::to_bootstrap_config()` is called
- **THEN** each phase is converted to its runtime equivalent via `From` implementations

### Requirement: Unused config fields SHALL be suppressed with #[expect()]

Fields defined for future implementations (network config values, security config values, clock NTP values, etc.) that have no runtime implementation SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]` (without phase references).

#### Scenario: Dead code lint does not fire for reserved fields

- **WHEN** `cargo clippy` runs on the bootstrap crate
- **THEN** no dead_code warnings for fields marked with `#[expect(dead_code, reason = "unsupported yet"]`
