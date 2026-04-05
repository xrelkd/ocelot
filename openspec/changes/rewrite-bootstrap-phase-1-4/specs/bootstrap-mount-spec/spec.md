## ADDED Requirements

### Requirement: MountSpec SHALL define a generic mount specification

The runtime `MountSpec` struct SHALL contain: `source: MountSource`, `target: PathBuf`, `fstype: &'static str`, `flags: MsFlags`, `options: Option<String>`, `overlay: Option<OverlaySpec>`, and `on_failure: MountFailurePolicy`.

#### Scenario: MountSpec with virtiofs source

- **WHEN** a MountSpec is created with `MountSource::VirtiofsTag("data")`
- **THEN** the mount operation uses virtiofs filesystem type with the specified tag

#### Scenario: MountSpec with custom flags

- **WHEN** a MountSpec has `flags: MsFlags::MS_NOSUID | MsFlags::MS_NODEV`
- **THEN** the mount syscall includes those flags

### Requirement: MountSource SHALL support multiple backend types

`MountSource` enum SHALL have variants: `Device(PathBuf)`, `VirtiofsTag(String)`, `NinePTag(String)`, `Virtual(String)`, `Nfs { server, path }`, and `Overlay { lower, upper, work }`.

#### Scenario: Block device mount source

- **WHEN** a MountSource is `Device("/dev/vda2")`
- **THEN** the mount operation uses the device path as the source

#### Scenario: NFS mount source (unsupported yet)

- **WHEN** a MountSource is `Nfs { server: "10.0.0.1", path: "/export" }`
- **THEN** the field is defined but marked `#[expect(dead_code, reason = "unsupported yet"]`

### Requirement: MountFailurePolicy SHALL define failure handling strategies

`MountFailurePolicy` enum SHALL have variants: `Abort`, `Warn`, and `Retry { max_attempts, backoff }`.

#### Scenario: Abort on mount failure

- **WHEN** a mount fails and policy is `Abort`
- **THEN** the bootstrap process returns an error immediately

#### Scenario: Retry with backoff (unsupported yet)

- **WHEN** a mount fails and policy is `Retry { max_attempts: 3, backoff: 1s }`
- **THEN** the field is defined but marked `#[expect(dead_code, reason = "unsupported yet"]`

### Requirement: MountSpecConfig SHALL be the serialization-layer counterpart

`MountSpecConfig` SHALL be a serde-tagged enum with variants matching `MountSource`, plus fields for `target`, `fstype`, `mountFlags` (string list), `options`, `overlay`, and `onFailure`.

#### Scenario: Deserialize mount spec with string flags

- **WHEN** YAML specifies `mountFlags: [nosuid, nodev]`
- **THEN** flags are converted to `MsFlags::MS_NOSUID | MsFlags::MS_NODEV`

#### Scenario: String-to-MsFlags conversion

- **WHEN** flag strings `ro`, `noexec`, `nosuid`, `nodev`, `relatime`, `strictatime` are parsed
- **THEN** each maps to the corresponding `MsFlags` variant

### Requirement: MountSpec SHALL support overlay specification

`OverlaySpec` (or reuse existing overlay logic) SHALL define lower/upper/work directories for overlay filesystems on top of mounts.

#### Scenario: Mount with overlay

- **WHEN** a MountSpec has `overlay: Some(...)`
- **THEN** an overlayfs is created on top of the base mount

### Requirement: Unused MountSource variants SHALL be suppressed

Variants not yet implemented (Nfs, Overlay as first-class) SHALL be marked with `#[expect(dead_code, reason = "unsupported yet"]`.

#### Scenario: Nfs variant does not trigger lint

- **WHEN** `cargo clippy` runs
- **THEN** no dead_code warning for `MountSource::Nfs` variant
