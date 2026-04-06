## ADDED Requirements

### Requirement: MountSpec SHALL define a generic mount specification

The runtime `MountSpec` struct SHALL contain: `source: MountSource`, `target: PathBuf`, `fstype: &'static str`, `flags: MsFlags`, `options: Option<String>`, `overlay: Option<OverlaySpec>`, and `on_failure: MountFailurePolicy`.

#### Scenario: MountSpec with virtiofs source

- **WHEN** a MountSpec is created with `MountSource::VirtiofsTag("data")`
- **THEN** the mount operation uses virtiofs filesystem type with the specified tag

#### Scenario: MountSpec with security flags

- **WHEN** a MountSpec has `flags` with `MsFlags::MS_RDONLY | MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID | MsFlags::MS_NODEV`
- **THEN** the mount is read-only, prevents execution, ignores setuid, and ignores device nodes

### Requirement: MountSource SHALL support multiple backend types

`MountSource` enum SHALL have variants: `Device(PathBuf)`, `VirtiofsTag(String)`, `NinePTag(String)`, `Virtual(String)`, `Nfs { server, path, fstype }`, and `Overlay { lower, upper, work }`.

#### Scenario: Block device mount source

- **WHEN** a MountSource is `Device("/dev/vda2")`
- **THEN** the mount operation uses the device path as the source

#### Scenario: NFS mount source (unsupported yet)

- **WHEN** a MountSource is `Nfs { server: "10.0.0.1", path: "/export", fstype: Some("nfs4") }`
- **THEN** the field is defined but marked `#[expect(dead_code, reason = "unsupported yet")`

### Requirement: MountSpecConfig SHALL use user-friendly boolean switches instead of string flag lists

`MountSpecConfig` enum SHALL have boolean fields for common mount flags instead of requiring users to specify flag strings. The flags SHALL be:

**Security booleans:**

- `read_only: bool` (default: `false`) → converts to `MsFlags::MS_RDONLY`
- `no_exec: bool` (default: `false`) → converts to `MsFlags::MS_NOEXEC`
- `no_suid: bool` (default: `false`) → converts to `MsFlags::MS_NOSUID`
- `no_dev: bool` (default: `false`) → converts to `MsFlags::MS_NODEV`

**Atime mode (mutually exclusive enum):**

- `atime: AtimeMode` (default: `AtimeMode::Default`) → converts to one of:
  - `AtimeMode::Default` → no flag (kernel default, typically relatime)
  - `AtimeMode::NoAtime` → `MsFlags::MS_NOATIME`
  - `AtimeMode::RelAtime` → `MsFlags::MS_RELATIME`
  - `AtimeMode::StrictAtime` → `MsFlags::MS_STRICTATIME`
  - `AtimeMode::LazyTime` → `MsFlags::MS_LAZYTIME`

**Other performance/behavior flags:**

- `sync: bool` (default: `false`) → `MsFlags::MS_SYNCHRONOUS`
- `dir_sync: bool` (default: `false`) → `MsFlags::MS_DIRSYNC`
- `mandatory_locks: bool` (default: `false`) → `MsFlags::MS_MANDLOCK`
- `posix_acl: bool` (default: `false`) → `MsFlags::MS_POSIXACL`

All boolean fields SHALL be `#[serde(default)]` so they default to `false` when omitted in YAML.

#### Scenario: YAML with boolean mount flags

- **WHEN** YAML specifies:
  ```yaml
  - type: virtiofs
    target: /mnt/data
    tag: data
    read_only: true
    no_exec: true
    no_suid: true
    no_dev: true
    atime: noatime
  ```
- **THEN** the conversion to `MountSpec` produces `flags` with `MS_RDONLY | MS_NOEXEC | MS_NOSUID | MS_NODEV | MS_NOATIME`

#### Scenario: Default values when flags omitted

- **WHEN** a mount spec omits all flag fields
- **THEN** `MsFlags::empty()` is used (no mount options)

#### Scenario: Invalid atime combination prevention

- **WHEN** a user tries to set conflicting atime options
- **THEN** the enum type prevents invalid combinations at compile time (YAML validation catches invalid enum values)

#### Scenario: Legacy string flags removed

- **GIVEN** the old `flags: Vec<String>` field existed
- **WHEN** the new design is implemented
- **THEN** the `flags` field is completely removed; no backward compatibility

### Requirement: AtimeMode enum SHALL be defined in serialization layer

`AtimeMode` enum SHALL be defined in `ocelot/src/config/bootstrap/mount/` with variants: `Default`, `NoAtime`, `RelAtime`, `StrictAtime`, `LazyTime`. The enum SHALL derive `Deserialize` with `#[serde(rename_all = "camelCase")]` so YAML can use `noatime`, `relAtime`, `strictAtime`, `lazyTime` as values.

#### Scenario: Deserialize atime mode from YAML

- **WHEN** YAML has `atime: noatime`
- **THEN** it deserializes to `AtimeMode::NoAtime`

### Requirement: Conversion SHALL build MsFlags from booleans

The `From<MountSpecConfig> for ocelot_bootstrap::MountSpec` implementation SHALL:

- Start with `MsFlags::empty()`
- OR-in `MS_RDONLY` if `read_only` is `true`
- OR-in `MS_NOEXEC` if `no_exec` is `true`
- OR-in `MS_NOSUID` if `no_suid` is `true`
- OR-in `MS_NODEV` if `no_dev` is `true`
- OR-in the appropriate atime flag based on `atime` (except `Default`)
- OR-in `MS_SYNCHRONOUS` if `sync` is `true`
- OR-in `MS_DIRSYNC` if `dir_sync` is `true`
- OR-in `MS_MANDLOCK` if `mandatory_locks` is `true`
- OR-in `MS_POSIXACL` if `posix_acl` is `true`
- Return the final `MsFlags` bitmask

#### Scenario: Multiple flags combined

- **WHEN** `read_only: true, no_exec: true, sync: true, atime: noatime`
- **THEN** resulting flags = `MS_RDONLY | MS_NOEXEC | MS_NOATIME | MS_SYNCHRONOUS`

### Requirement: MountFailurePolicy SHALL define failure handling strategies

`MountFailurePolicy` enum SHALL have variants: `Abort`, `Warn`, and `Retry { max_attempts, backoff }`.

#### Scenario: Abort on mount failure

- **WHEN** a mount fails and policy is `Abort`
- **THEN** the bootstrap process returns an error immediately

#### Scenario: Retry with backoff (unsupported yet)

- **WHEN** a mount fails and policy is `Retry { max_attempts: 3, backoff: 1s }`
- **THEN** the field is defined but marked `#[expect(dead_code, reason = "unsupported yet")`

### Requirement: Unused MountSource variants SHALL be suppressed

Variants not yet implemented (Nfs, Overlay as first-class) SHALL be marked with `#[expect(dead_code, reason = "unsupported yet")`.

#### Scenario: Nfs variant does not trigger lint

- **WHEN** `cargo clippy` runs
- **THEN** no dead_code warning for `MountSource::Nfs` variant
