# Mount Flags Boolean Switch Refinement

## Problem

The original `MountSpecConfig` design used `flags: Vec<String>` requiring users to know Linux kernel constant names like `MS_RDONLY`, `MS_NOEXEC`, etc. This was:

- Unuser-friendly (cryptic names, easy to typo with silent ignore)
- Not discoverable (no IDE autocomplete)
- Required memorization of kernel constants

## Solution

Replace the string-based `flags` field with explicit boolean switches for common use cases plus an `AtimeMode` enum for atime configuration.

## Changes

### Serialization Layer (`ocelot/src/config/bootstrap/mount/`)

Add to **all** `MountSpecConfig` enum variants:

```rust
#[serde(default)]
pub read_only: bool,        // false → MS_RDONLY when true
#[serde(default)]
pub no_exec: bool,          // false → MS_NOEXEC when true
#[serde(default)]
pub no_suid: bool,          // false → MS_NOSUID when true
#[serde(default)]
pub no_dev: bool,           // false → MS_NODEV when true
#[serde(default)]
pub sync: bool,             // false → MS_SYNCHRONOUS when true
#[serde(default)]
pub dir_sync: bool,         // false → MS_DIRSYNC when true
#[serde(default)]
pub mandatory_locks: bool,  // false → MS_MANDLOCK when true
#[serde(default)]
pub posix_acl: bool,        // false → MS_POSIXACL when true
#[serde(default)]
pub atime: AtimeMode,       // Default → no flag; others map to atime flags
```

**NOTE on naming:** The struct field names use Rust's `snake_case` convention. However, to enable YAML to use `readOnly`, `noExec`, `noSuid`, `noDev`, `dirSync`, `mandatoryLocks`, `posixAcl`, the `MountSpecConfig` enum (or the containing module) SHALL have `#[serde(rename_all = "camelCase")]` applied. Alternatively, each field can have `#[serde(rename = "camelCaseName")]` individually. The convention is:

- Struct fields: `snake_case` (Rust convention)
- YAML keys: `camelCase` (matches typical YAML style and is more readable)

The `AtimeMode` enum already uses `#[serde(rename_all = "camelCase")]` so its variants (`noAtime`, `relAtime`, `strictAtime`, `lazyTime`) are in camelCase.

Define `AtimeMode` enum:

```rust
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AtimeMode {
    Default,      // kernel default (usually relatime)
    NoAtime,      // MS_NOATIME
    RelAtime,     // MS_RELATIME
    StrictAtime,  // MS_STRICTATIME
    LazyTime,     // MS_LAZYTIME
}
```

Remove the old `flags: Option<Vec<String>>` field entirely.

### Conversion Logic (`From<MountSpecConfig> for MountSpec`)

Build `MsFlags` bitmask:

```rust
let mut flags = MsFlags::empty();
if config.read_only { flags |= MsFlags::MS_RDONLY; }
if config.no_exec { flags |= MsFlags::MS_NOEXEC; }
if config.no_suid { flags |= MsFlags::MS_NOSUID; }
if config.no_dev { flags |= MsFlags::MS_NODEV; }
if config.sync { flags |= MsFlags::MS_SYNCHRONOUS; }
if config.dir_sync { flags |= MsFlags::MS_DIRSYNC; }
if config.mandatory_locks { flags |= MsFlags::MS_MANDLOCK; }
if config.posix_acl { flags |= MsFlags::MS_POSIXACL; }
flags |= match config.atime {
    AtimeMode::Default => MsFlags::empty(),
    AtimeMode::NoAtime => MsFlags::MS_NOATIME,
    AtimeMode::RelAtime => MsFlags::MS_RELATIME,
    AtimeMode::StrictAtime => MsFlags::MS_STRICTATIME,
    AtimeMode::LazyTime => MsFlags::MS_LAZYTIME,
};
```

### Default Values

All boolean fields default to `false` (via `#[serde(default)]`), meaning the corresponding flag is **not** set.
`atime` defaults to `AtimeMode::Default`, meaning no atime flag is set (kernel uses its default).

These defaults produce `MsFlags::empty()` when no flag fields are specified.

## Example YAML

```yaml
mounts:
  - type: virtiofs
    target: /mnt/data
    tag: data
    readOnly: true
    noExec: true
    noSuid: true
    noDev: true
    atime: noAtime
```

## Integration

- Update `ocelot/src/config/bootstrap/mount/spec.rs`
- Update conversion in `impl From<MountSpecConfig> for MountSpec`
- Update templates to show new flag fields
- Run `cargo clippy-all` to ensure no warnings
- No backward compatibility needed (configs are brand new)

## Rationale for Exposed Flags

These flags cover the common use cases for both VM and physical machine deployments:

- **Security**: `read_only`, `no_exec`, `no_suid`, `no_dev` — the standard security triad plus read-only
- **Performance**: `atime` modes (reduce disk I/O), `sync`/`dir_sync` for data integrity (rare)
- **Advanced features**: `mandatory_locks`, `posix_acl` for special requirements

Flags NOT exposed (internal use only):

- `MS_REC`, `MS_MOVE` — used internally by bootstrap
- `MS_BIND` — implicit in bind mounts
- `MS_REMOUNT` — separate operation
- `MS_SHARED`, `MS_PRIVATE`, `MS_SLAVE`, `MS_UNBINDABLE` — mount propagation, internal
- `MS_SILENT`, `MS_I_VERSION` — kernel internals

## Atime Enum Justification

Atime-related flags (`MS_NOATIME`, `MS_RELATIME`, `MS_STRICTATIME`, `MS_LAZYTIME`) are **mutually exclusive**. Using separate booleans would allow invalid combinations (e.g., `noatime: true` and `relatime: true`). An enum prevents this at the type level and makes YAML validation catch errors early.
