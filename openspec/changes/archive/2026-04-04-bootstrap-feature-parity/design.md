## Context

The `ocelot-bootstrap` crate provides a structured, config-file-driven boot flow for QEMU VMs. It currently supports a single root filesystem (virtiofs, block, or 9p), explicit module-by-name loading, console setup, switch_root, and handoff to the supervise orchestrator.

Additional capabilities are needed in practice: multiple virtiofs shares, per-share overlayfs, directory-scanning module loading, virtiofs pre-flight checks, symlink creation, and boot script execution. These are currently addressed through ad-hoc approaches. This change brings them into bootstrap with structured config support.

Constraints:

- Bootstrap must remain a library crate (not standalone binary)
- Existing config YAML schema must remain backward-compatible
- Must use workspace conventions: snafu errors, nix crate, strict lints
- Boot flow order is fixed: console → VFS → modules → root → overlay → extras → symlinks → env → script → switch_root → handoff

## Goals / Non-Goals

**Goals:**

- Add multiple virtiofs mount support with per-share overlayfs
- Add directory-scanning kernel module loading mode
- Add virtiofs support detection before mount attempts
- Add symlink creation during boot flow
- Add optional boot script execution
- All new features configurable via existing YAML config file
- Zero breaking changes to existing config fields

**Non-Goals:**

- Rich cmdline parsing (backtick syntax) — config file only
- Dynamic boot stage ordering — fixed sequence
- Plugin/pipeline architecture — additive changes only
- Block device improvements (udev, fallback strategies) — out of scope

## Decisions

### 1. Extra virtiofs mounts as separate field, not unified mount list

**Decision**: Add `extra_virtiofs_mounts: Vec<VirtiofsMount>` to `Config` rather than replacing `RootConfig` with a unified mount list.

**Rationale**: The root filesystem is conceptually different from additional shares. The root determines where switch_root targets; extra mounts are mounted under the new root. A unified list would blur this distinction and complicate switch_root logic.

**Alternatives considered**:

- Unified `Vec<MountSpec>` with `is_root: bool` flag — more flexible but requires refactoring switch_root to identify which mount is the root. Over-engineered for current needs.

### 2. ModulesConfig as enum, not flat struct

**Decision**: Convert `ModulesConfig` from a flat struct to an enum:

```rust
pub enum ModulesConfig {
    /// Load specific modules by name.
    /// When `dir` is `None`, defaults to `/lib/modules`.
    List { dir: Option<String>, names: Vec<String> },
    /// Scan directory for all .ko/.ko.xz/.ko.gz files.
    Scan { dir: String },
}
```

**Rationale**: The two modes are mutually exclusive (you either list specific modules OR scan a directory). An enum makes this explicit at the type level and prevents invalid states (both dir+list and scan semantics active simultaneously). The name `ModulesConfig` is retained for backward compatibility with the existing config field name.

**Alternatives considered**:

- Add `scan_dir: Option<String>` alongside existing fields — allows ambiguous configs where both list and scan_dir are set.
- Replace entirely with scan mode — loses the ability to load specific modules in a controlled order.
- New name `ModuleMode` — rejected; `ModulesConfig` matches the existing YAML key and is clearer about its role.

### 3. Separate config types for CLI and library

**Decision**: CLI config structs in `ocelot/src/config/bootstrap.rs` and library config structs in `crates/bootstrap/src/config.rs` are completely separate. Library types do NOT derive `serde::Deserialize`. CLI types handle YAML parsing and convert to library types via `From`/`TryFrom` implementations.

**Rationale**: The library crate should not depend on serialization frameworks. Its config types are pure data structures used by the boot flow. Serde derives belong only in the CLI layer where YAML is actually parsed. This keeps the library dependency graph minimal and allows the two layers to evolve independently (e.g., different field naming conventions, additional CLI-only validation).

**Pattern**:

```rust
// CLI layer (ocelot/src/config/bootstrap.rs) — derives Deserialize
#[derive(Deserialize)]
pub struct ModulesConfig { /* serde fields */ }

// Library layer (crates/bootstrap/src/config.rs) — no serde
pub enum ModulesConfig {
    List { dir: Option<String>, names: Vec<String> },
    Scan { dir: String },
}

// Conversion
impl From<crate::config::ModulesConfig> for ocelot_bootstrap::ModulesConfig { ... }
```

### 4. Per-share overlayfs uses `/run/overlayfs/{tag}/` isolation

**Decision**: Each extra virtiofs mount with `with_overlay: true` gets its own overlay structure under `/run/overlayfs/{tag}/`.

**Rationale**: Tags are unique identifiers. Using them as directory names provides natural isolation. The root filesystem overlay already uses `/run/overlayfs/{source}/` with sanitized source names — same pattern, different namespace.

### 5. Symlinks created after switch_root, not before

**Decision**: Symlink creation happens after `chroot` into the new root, not in the initramfs.

**Rationale**: Symlinks are meant to affect the target root filesystem, not the initramfs. Creating them before switch_root would create symlinks in the wrong filesystem.

### 6. Boot script runs after env vars, before handoff

**Decision**: Boot script executes after environment variables and working directory are set, but before switch_root hands off to supervise or shell. Script execution uses `entry::execute` (not `std::process::Command`).

**Rationale**: `entry::execute` provides zombie reaping, signal forwarding, and optional timeout — all important during boot. If the boot script spawns children that exit, zombies accumulate without reaping. `entry::execute` already handles this via SIGCHLD handling. The overhead (epoll + pipes) is negligible. `entry::execute_interactive` is not used since boot scripts don't need console/tty setup.

### 7. Virtiofs support check runs once, before any virtiofs mount

**Decision**: Single pre-flight check of `/proc/filesystems` before the first virtiofs mount attempt (root or extra). Not repeated for each mount.

**Rationale**: If the kernel supports virtiofs, it supports it for all mounts. Repeated checks add no value. The check is cheap but unnecessary to repeat.

## Risks / Trade-offs

| Risk                                                           | Mitigation                                                                                                                    |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| Extra virtiofs mounts fail mid-sequence, leaving partial state | Log each mount attempt; failures are warnings, not fatal (matching existing module loading behavior)                          |
| Module scan loads unwanted modules from directory              | Scan mode is opt-in via config; users control which directory is scanned                                                      |
| Boot script exits with non-zero code                           | Configurable: `boot_script.on_failure: "warn"                                                                                 | "abort"` (default: warn) |
| Symlink target doesn't exist yet                               | Create parent directories if needed; log warning if target is missing (symlink still created)                                 |
| Config file grows large with many extra mounts                 | Not a technical risk, but users with 10+ shares may want to split config. Out of scope for now.                               |
| Per-share overlayfs consumes tmpfs memory for upper layers     | Each overlay's upperdir lives on tmpfs (`/run`). Users with many writable shares need adequate RAM. Document this limitation. |
