## Context

Ocelot's bootstrap crate currently handles initramfs → PID 1 handoff with a flat configuration structure and an incorrect boot flow. The `switch_root` function uses `chroot` instead of `pivot_root`, boot scripts execute after supervise exits (completely wrong timing), and the configuration cannot express which operations must happen before vs. after the root switch.

Additionally, the current configuration validation does not properly validate module dependencies and process dependencies, and deprecated configuration structures remain in the codebase causing confusion.

The analysis document identifies 8 phases of a complete boot process, of which only phases 0, 1, 3, 5, and 7 are partially implemented. The current flat `BootstrapConfig` has no concept of pre/post switch timing.

**Constraints:**

- Independent configuration files must be created in `ocelot/src/config/` for better organization
- Deprecated configuration structs and enums must be dropped entirely (no `#[expect(dead_code)]` needed)
- New configuration structs must be placed in `ocelot/src/config/` with independent files
- Validation must apply module dependency detection and process dependency detection functions
- Fully qualified names must be used for nix crate functions (e.g., `nix::mount::mount()`)
- Remove all " — reserved for Phase X" descriptions from comments
- No migration path needed — configs can be rewritten from scratch
- Unused items must have `#[expect()]` with reason for lint suppression (only for truly unimplemented features, not deprecated ones)

## Goals / Non-Goals

**Goals:**

- Remove deprecated configuration structs and enums from bootstrap configuration
- Create new independent configuration files in `ocelot/src/config/` for better organization
- Restructure config into `preSwitch` / `switchRoot` / `postSwitch` phases
- Replace `chroot` with proper `pivot_root` in switch_root
- Fix boot script execution timing (before handoff, not after)
- Create `phase/` module with `pre()`/`post()` per subsystem
- Introduce `MountSpec` abstraction with source types, flags, and failure policies
- Add mount namespace isolation (`MS_REC | MS_PRIVATE`)
- Add missing virtual filesystems (`/dev/pts`, `/dev/shm`, `/tmp`)
- Enhance validation to apply module dependency detection and process dependency detection
- Use fully qualified names for nix crate functions
- All unused code properly suppressed with `#[expect()]` + reason (only for truly unimplemented features)

**Non-Goals:**

- LUKS/LVM/NFS root support (Phase 8 — deferred)
- Network configuration implementation (DHCP/static) — types defined, implementation stubbed
- Migration from old config format
- systemd compatibility or dracut module integration
- SELinux/AppArmor enforcement (types defined, implementation stubbed)
- NTP time synchronization (types defined, implementation stubbed)

## Decisions

### 1. Independent configuration files

**Decision:** Move `BootstrapConfig` and related types to independent files in `ocelot/src/config/` instead of keeping them in a single file.

**Rationale:** Better organization and separation of concerns. Each configuration type gets its own logical grouping.

**Alternatives considered:**

- Keep all configuration in one file — rejected for poor organization
- Split by phase within one file — rejected, still mixes concerns

### 2. Drop deprecated configurations entirely

**Decision:** Remove deprecated configuration structs and enums completely rather than marking them with `#[expect(dead_code)]`.

**Rationale:** The project explicitly stated not to consider migration, so deprecated code should be removed entirely to reduce confusion and code size.

**Alternatives considered:**

- Mark deprecated items with `#[expect(dead_code)]` — rejected per user request to drop them entirely
- Keep deprecated items for potential migration — rejected per user request

### 3. Module and process dependency validation

**Decision:** Integrate module dependency detection and process dependency detection functions directly into the validation process for `BootstrapConfig`.

**Rationale:** Ensures configuration validity at load time rather than runtime, preventing boot failures due to misordered modules or missing processes.

**Alternatives considered:**

- Validate dependencies at runtime — rejected, could cause boot failures
- Separate validation step — rejected, less integrated approach

### 4. Fully qualified nix function names

**Decision:** Use fully qualified names for all nix crate functions (e.g., `nix::mount::mount()`, `nix::unistd::pivot_root()`) instead of `use` statements.

**Rationale:** Improves code clarity by making it immediately obvious which functions come from the nix crate, avoiding namespace confusion.

**Alternatives considered:**

- Use `use nix::mount; use nix::unistd;` — rejected per user request for fully qualified names
- Use `use` statements with renaming — rejected, still not fully qualified

### 5. Phase module structure

**Decision:** Create `crates/bootstrap/src/phase/` with one file per subsystem, each exporting `pre()` and/or `post()` functions.

**Rationale:** Keeps `lib.rs` as a coordinator, subsystem modules (`mount.rs`, `modules.rs`) pure, and phase logic centralized. Function naming: `clock::pre()`, `clock::post()`, etc. — no `phase_` prefix.

**Alternatives considered:**

- Inline all phase logic in `lib.rs` — rejected, would bloat the file
- One file per phase (pre.rs, post.rs) — rejected, would mix unrelated subsystems

### 6. switch_root split into `only()` + handoff

**Decision:** Split `switch_root()` into `switch_root::only(config)` (pivot_root only) and separate handoff call in `lib.rs`.

**Rationale:** Current implementation bundles pivot_root + exec into one function, making it impossible to run boot scripts at the right time. The split allows: `only()` → boot_script → `exec_supervise()`.

**Alternatives considered:**

- Keep single function with callback — rejected, callback type would be complex
- Add boolean flag to skip exec — rejected, unclear semantics

### 7. MountSpec abstraction

**Decision:** Introduce `MountSpec` (runtime) and `MountSpecConfig` (serialization) as a unified mount specification with `MountSource` enum, `MsFlags`, and `MountFailurePolicy`.

**Rationale:** Current hardcoded virtiofs/block/9p paths don't scale. The abstraction supports future filesystem types (NFS, overlay as first-class) without touching mount logic.

**Alternatives considered:**

- Keep per-backend functions — rejected, doesn't support generic mount ordering
- Use trait-based mount backend — overengineered for current needs

### 8. Execution order

**Decision:** preSwitch order: `virtual_filesystems → clock::pre → sysctl::pre → tmpfiles::pre → symlinks::pre → environment::pre → modules::pre → network::pre → mounts::pre → hooks::pre`. postSwitch order: reverse with handoff at end.

**Rationale:** Virtual filesystems first (proc/sys/dev needed by everything). Modules before mounts (drivers needed for filesystem access). Mounts before hooks (hooks may need mounted filesystems). Post-switch roughly reverses to set up runtime environment before handoff.

## Risks / Trade-offs

- **[Config breakage]** All existing YAML configs are invalid after this change → Mitigation: templates are updated, no migration path needed per project decision
- **[pivot_root compatibility]** Some environments may not support `pivot_root` (e.g., containers) → Mitigation: config allows `method: chroot` fallback (though this should be avoided)
- **[Binary size growth]** New phase module and types increase bootstrap binary → Mitigation: unused code is properly stripped; actual runtime code is minimal
- **[Validation complexity]** Enhanced validation adds complexity to config loading → Mitigation: centralizes important safety checks that prevent boot failures
- **[Name verbosity]** Fully qualified nix names are more verbose → Mitigation: improved clarity outweighs verbosity, IDEs can help with completion

## Refinements

### Mount Flags: User-Friendly Boolean Switches

**Problem:** The original `MountSpecConfig` used `flags: Vec<String>` requiring users to know Linux kernel constant names (`MS_RDONLY`, `MS_NOEXEC`, etc.). This was unuser-friendly, not discoverable, and typos were silently ignored.

**Solution:** Replace string flags with explicit boolean fields for common use cases:

- `read_only`, `no_exec`, `no_suid`, `no_dev` (security)
- `sync`, `dir_sync`, `mandatory_locks`, `posix_acl` (advanced)
- `atime: AtimeMode` enum (`Default`, `NoAtime`, `RelAtime`, `StrictAtime`, `LazyTime`)

All fields are `#[serde(default)]` with defaults: `false` for booleans, `AtimeMode::Default` for atime.

**YAML naming:** Although Rust fields use `snake_case`, YAML keys use `camelCase` via `#[serde(rename_all = "camelCase")]`. So in YAML you write: `readOnly`, `noExec`, `noSuid`, `noDev`, `dirSync`, `mandatoryLocks`, `posixAcl`.

**Conversion:** Build `MsFlags` bitmask by OR-ing bits from boolean fields. No escape hatch — all supported flags are explicitly enumerated.

**Rationale for exposed flags:** Covers security (ro, noexec, nosuid, nodev) and performance tuning (atime modes, sync). Internal flags (MS_REC, MS_MOVE, propagation flags) remain bootstrap-internal.

**Atime as enum:** Atime flags are mutually exclusive; enum prevents invalid combinations at compile/YAML validation time.
