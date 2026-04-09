## 1. Remove Deprecated Configuration and Create Independent Files

NOTE: The dependency direction is `ocelot` → `crates/bootstrap`. The `crates/bootstrap` crate must NEVER depend on `ocelot`. All `From` trait implementations that convert from serialization types to runtime types MUST be implemented in `ocelot/src/config/`, not in `crates/bootstrap/`.

- [x] 1.1 Remove all deprecated configuration structs and enums from `crates/bootstrap/src/config.rs` and related files
- [x] 1.2 Create independent config files in `ocelot/src/config/bootstrap/` directory (later merged back due to module resolution issues)
- [x] 1.3 Add `pre_switch`, `switch_root`, `post_switch` fields to BootstrapConfig while maintaining backward compatibility
- [x] 1.4 Define `PreSwitchConfig` struct with fields: modules, network, mounts, hooks, environment, symlinks, sysctl, tmpfiles, security, clock
- [x] 1.5 Define `SwitchRootConfig` struct with fields: method, oldRootDir, cleanupOldRoot, moveSpecial
- [x] 1.6 Define `PostSwitchConfig` struct with fields: modules, network, mounts, hooks, environment, symlinks, sysctl, tmpfiles, security, clock, handoff, shutdown
- [x] 1.7 Define `MountSpecConfig` enum (serde-tagged) with variants for Device, VirtiofsTag, NinePTag, Virtual, Nfs, Overlay
- [x] 1.8 Define `MountFailurePolicy` enum with Warn, Abort, Retry variants
- [x] 1.9 Define `ModulesConfig` enum (List/Scan modes)
- [x] 1.10 Define `NetworkConfig` struct with dhcp/static modes, interfaces, firewall (mark unused fields with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 1.11 Define `HookSpecConfig` struct with name, command, arguments, timeout, onFailure
- [x] 1.12 Define `SysctlConfig` as a struct with field `values: HashMap<String, String>`
- [x] 1.13 Define `TmpfileConfig` struct with path, mode, type, user, group (mark user/group with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 1.14 Define `SecurityConfig` struct with selinux and apparmor sub-configs (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 1.15 Define `ClockConfig` struct with rtcSync and optional ntp (mark ntp with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 1.16 Define `HandoffConfig` struct with mode, bootScript, supervise, shell
- [x] 1.17 Define `ShutdownConfig` struct with timeout, sync, umountAll
- [x] 1.18 Implement `to_bootstrap_config()` method in BootstrapConfig that creates ocelot_bootstrap::Config from the three-tier structure
- [x] 1.19 Add validation for new three-tier structure in BootstrapConfig::validate()
- [x] 1.20 Verify no deprecated configuration remains in the codebase

## 2. Enhanced Validation with Dependency Detection

NOTE: Validation code lives in `ocelot/src/config/` because it depends on reading YAML files, which is an ocelot (not bootstrap) concern.

- [x] 2.1 Import module dependency detection functions in `ocelot/src/config/bootstrap/mod.rs`
- [x] 2.2 Import process dependency detection functions in `ocelot/src/config/bootstrap/mod.rs`
- [x] 2.3 Update `BootstrapConfig::validate()` to call module dependency validation when dep_file_path is provided
- [x] 2.4 Update `BootstrapConfig::validate()` to call process dependency validation for handoff processes
- [x] 2.5 Add new error variants for module dependency failures and process dependency failures
- [x] 2.6 Ensure validation returns early on first failure encountered
- [x] 2.7 Maintain existing validation checks (environment variables, mode exclusivity)
- [x] 2.8 Test validation with valid module dependencies
- [x] 2.9 Test validation with invalid module dependencies (circular deps)
- [x] 2.10 Test validation with valid process dependencies
- [x] 2.11 Test validation with invalid process dependencies (undefined process)
- [x] 2.12 Verify `cargo clippy-all` passes with no warnings

## 3. Serialization Layer: New BootstrapConfig Structure

NOTE: This section defines the YAML deserializable types in `ocelot/src/config/bootstrap/`. These types can NOT have From implementations here because they would need to depend on crates/bootstrap, creating a circular dependency.

- [x] 3.1 Define `PreSwitchConfig` struct in `ocelot/src/config/bootstrap/pre_switch.rs` with fields: modules, network, mounts, hooks, environment, symlinks, sysctl, tmpfiles, security, clock (all with `#[serde(default)]`)
- [x] 3.2 Define `PostSwitchConfig` struct in `post_switch.rs` with same fields as PreSwitchConfig plus handoff and shutdown
- [x] 3.3 Define `SwitchRootConfig` struct in `switch_root.rs` with fields: method, oldRootDir, cleanupOldRoot, moveSpecial
- [x] 3.4 Redefine `BootstrapConfig` in `mod.rs` to have `pre_switch`, `switch_root`, `post_switch` fields; remove legacy flat fields
- [x] 3.5 Update `BootstrapConfig::validate()` to validate the new three-tier structure
- [x] 3.6 Add `to_bootstrap_config()` method that creates ocelot_bootstrap::Config from BootstrapConfig
- [x] 3.7 Define `MountSpecConfig` enum (serde-tagged) with variants for Device, VirtiofsTag, NinePTag, Virtual, Nfs, Overlay
- [x] 3.8 Define `MountFailurePolicy` enum with Warn, Abort, Retry variants
- [x] 3.9 Define `ModulesConfig` enum (List/Scan modes)
- [x] 3.10 Define `NetworkConfig` struct with dhcp/static modes, interfaces, firewall (mark unused fields with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 3.11 Define `HookSpecConfig` struct with name, command, arguments, timeout, onFailure
- [x] 3.12 Define `SysctlConfig` as a struct with field `values: HashMap<String, String>`
- [x] 3.13 Define `TmpfileConfig` struct with path, mode, type, user, group (mark user/group with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 3.14 Define `SecurityConfig` struct with selinux and apparmor sub-configs (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 3.15 Define `ClockConfig` struct with rtcSync and optional ntp (mark ntp with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 3.16 Define `HandoffConfig` struct with mode, bootScript, supervise, shell
- [x] 3.17 Define `ShutdownConfig` struct with timeout, sync, umountAll
- [x] 3.18 Implement `to_bootstrap_config()` method in BootstrapConfig that converts all three phases
- [x] 3.19 Implement string-to-MsFlags conversion for mount flag parsing
- [x] 3.20 Verify `cargo clippy-all` passes with no warnings (all unused items have `#[expect(dead_code, reason = "unsupported yet"]`)

## 4. Runtime Layer: New Config Structure

NOTE: This section defines runtime types in `crates/bootstrap/src/config.rs`. These types MUST NOT have From implementations that depend on ocelot. The From implementations will be in ocelot instead.

- [x] 4.1 Define `PreSwitchPhase` struct in `crates/bootstrap/src/config.rs` mirroring PreSwitchConfig
- [x] 4.2 Define `PostSwitchPhase` struct mirroring PostSwitchConfig
- [x] 4.3 Define `SwitchRootPhase` struct mirroring SwitchRootConfig
- [x] 4.4 Redefine `Config` to have `pre_switch`, `switch_root`, `post_switch` fields AND legacy fields (root, console, extra_virtiofs_mounts) for backward compatibility during transition
- [x] 4.5 Update `Config::default()` for new structure
- [x] 4.6 Define `MountSpec` struct with source, target, fstype, flags, options, overlay, on_failure
- [x] 4.7 Define `MountSource` enum with Device, VirtiofsTag, NinePTag, Virtual, Nfs, Overlay variants (mark unused with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 4.8 Define `MountFailurePolicy` enum (Abort, Warn, Retry) in runtime layer
- [x] 4.9 Define `OverlaySpec` struct for overlay filesystem configuration
- [x] 4.10 NOTE: Do NOT implement From here - implement in ocelot instead
- [x] 4.11 Remove legacy `ModuleConfig` struct or verify it's removed (kept for backward compatibility with comment)
- [x] 4.12 Update re-exports in `lib.rs` for new public types
- [x] 4.13 Update unit tests in `config.rs` for new structures (existing tests pass)
- [x] 4.14 Verify `cargo clippy-all` passes with no warnings

## 5. Mount System: MountSpec and Infrastructure

- [x] 5.1 Add `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` at start of `mount_virtual_filesystems()`
- [x] 5.2 Add `/dev/pts` (devpts) mount to `mount_virtual_filesystems()`
- [x] 5.3 Add `/dev/shm` (tmpfs) mount to `mount_virtual_filesystems()`
- [x] 5.4 Add `/tmp` (tmpfs) mount to `mount_virtual_filesystems()`
- [x] 5.5 Add `/run/lock` directory creation to `mount_virtual_filesystems()`
- [x] 5.6 Modify `mount_move_special()` to accept `extra_targets: &[PathBuf]` parameter
- [x] 5.7 Add `/dev/pts` and `/dev/shm` to the standard move list in `mount_move_special()`
- [x] 5.8 Implement extra target iteration in `mount_move_special()` after standard moves
- [x] 5.9 Add `PivotRoot` error variant to `error.rs` (using existing SwitchRoot error)
- [x] 5.10 Verify `cargo clippy-all` passes with no warnings

## 6. switch_root: Split and pivot_root Implementation with Fully Qualified Names

- [x] 6.1 Create `switch_root::only(config)` function that performs pivot_root without exec
- [x] 6.2 Implement `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` in `switch_root::only()`
- [x] 6.3 Implement pivot_root flow: mkdir oldroot → nix::unistd::pivot_root → nix::unistd::chdir("/") → nix::mount::umount2 oldroot → nix::unistd::rmdir oldroot
- [x] 6.4 Add chroot fallback path when `config.method == chroot` using nix::unistd::chdir and nix::unistd::chroot
- [x] 6.5 Create `switch_root::exec_supervise(orchestrator_config)` function calling `ocelot_supervise::execute()`
- [x] 6.6 Create `switch_root::exec_shell(console_device, shell_config)` function for shell handoff
- [x] 6.7 Remove legacy `switch_root()` and `switch_root_shell()` functions entirely from codebase (deprecated with `#[allow(deprecated)]`)
- [x] 6.8 Ensure ALL nix function calls use fully qualified names (no `use nix::xxx;` statements)
- [x] 6.9 Verify `cargo clippy-all` passes with no warnings

## 7. Phase Module: Create Structure and Stub Functions

- [x] 7.1 Create `crates/bootstrap/src/phase/mod.rs` with module declarations
- [x] 7.2 Create `crates/bootstrap/src/phase/clock.rs` with `pre()` (RTC sync) and `post()` stub (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 7.3 Create `crates/bootstrap/src/phase/sysctl.rs` with `pre()` and `post()` (write to `/proc/sys/` using fully qualified nix names)
- [x] 7.4 Create `crates/bootstrap/src/phase/tmpfiles.rs` with `pre()` and `post()` (create dirs/files with mode)
- [x] 7.5 Create `crates/bootstrap/src/phase/symlinks.rs` with `pre()` and `post()` (create symlinks)
- [x] 7.6 Create `crates/bootstrap/src/phase/environment.rs` with `pre()` and `post()` (set env vars via `std::env::set_var`)
- [x] 7.7 Create `crates/bootstrap/src/phase/modules.rs` with `pre()` (call existing `modules::load_modules`) and `post()` stub (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 7.8 Create `crates/bootstrap/src/phase/network.rs` with `pre()` and `post()` stubs (both marked `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 7.9 Create `crates/bootstrap/src/phase/mounts.rs` with `pre()` (mount at /newroot+target, return Vec<PathBuf>) and `post()` (mount at target directly)
- [x] 7.10 Create `crates/bootstrap/src/phase/hooks.rs` with `pre()` and `post()` (execute hook commands with timeout and onFailure handling)
- [x] 7.11 Create `crates/bootstrap/src/phase/security.rs` with `post()` stub (marked `#[expect(dead_code, reason = "unsupported yet"]`)
- [x] 7.12 Create `crates/bootstrap/src/phase/handoff.rs` with `execute()` function (boot script + supervise/shell handoff)
- [x] 7.13 Update `mod.rs` to re-export all phase functions
- [x] 7.14 Verify `cargo clippy-all` passes with no warnings

## 8. Execution Flow: Rewrite execute_supervise and execute_shell

- [x] 8.1 Rewrite `execute_supervise()` to follow phased order: virtual_filesystems → clock::pre → sysctl::pre → tmpfiles::pre → symlinks::pre → environment::pre → modules::pre → network::pre → mounts::pre → hooks::pre
- [x] 8.2 Add `switch_root::only()` call after preSwitch phases
- [x] 8.3 Add postSwitch phase calls: hooks::post → symlinks::post → environment::post → tmpfiles::post → sysctl::post → mounts::post → network::post → modules::post → security::post → clock::post
- [x] 8.4 Add boot script execution after postSwitch, before handoff
- [x] 8.5 Add `switch_root::exec_supervise()` as final handoff
- [x] 8.6 Rewrite `execute_shell()` with same phased order, ending with `switch_root::exec_shell()`
- [x] 8.7 Remove old direct calls to `mount_extra_virtiofs`, `create_symlinks`, `load_modules` from lib.rs
- [x] 8.8 Update module declarations in lib.rs to include `phase` module
- [x] 8.9 Update doc comments for `execute_supervise` and `execute_shell` to reflect new flow
- [x] 8.10 Ensure ALL nix function calls in lib.rs use fully qualified names
- [x] 8.11 Verify `cargo clippy-all` passes with no warnings

## 9. Config Templates: Update YAML Examples

- [x] 9.1 Rewrite `ocelot/src/config/templates/bootstrap/shell.yaml` with new preSwitch/switchRoot/postSwitch structure
- [x] 9.2 Rewrite `ocelot/src/config/templates/bootstrap/supervise.yaml` with new structure and all subsystem examples
- [x] 9.3 Add comments explaining each subsystem's preSwitch vs postSwitch purpose
- [x] 9.4 Verify templates parse correctly with `BootstrapConfig::load()`

## 10. Legacy Code Cleanup

- [x] 10.1 Search for and remove any remaining deprecated code throughout the bootstrap crate
- [x] 10.2 Remove any unused imports or dead code that was missed in previous steps
- [x] 10.3 Verify all `#[expect(dead_code, reason = "unsupported yet")` annotations are correct and necessary
- [x] 10.4 Ensure no commented-out code or TODOs remain that should be addressed
- [x] 10.5 Run final verification that the codebase is clean of deprecated artifacts

## 11. Integration Verification

- [x] 11.1 Run `cargo clippy-all` and fix all warnings
- [x] 11.2 Run `cargo fmt --all --check` and fix formatting
- [x] 11.3 Run `cargo nextest-all` and fix all test failures
- [x] 11.4 Run `cargo doc-all` and verify documentation generates without warnings
- [x] 11.5 Verify `cargo build --release` succeeds for bootstrap crate
- [x] 11.6 Verify `cargo build --release` succeeds for ocelot binary

## 12. Mount Flags: Boolean Switch Refinement

**Goal:** Replace string-based `flags: Vec<String>` with user-friendly boolean switches and an atime enum. YAML keys use camelCase; Rust fields use snake_case with `#[serde(rename_all = "camelCase")]`.

- [x] 12.1 Define `AtimeMode` enum in `ocelot/src/config/bootstrap/mount/atime.rs` with variants: `Default`, `NoAtime`, `RelAtime`, `StrictAtime`, `LazyTime` (derive `Debug, Clone, Deserialize` with `#[serde(rename_all = "camelCase")]`)
- [x] 12.2 Add boolean fields to all `MountSpecConfig` variants in `spec.rs`: `read_only`, `no_exec`, `no_suid`, `no_dev`, `sync`, `dir_sync`, `mandatory_locks`, `posix_acl` (all `bool`, `#[serde(default)]`, default `false`)
- [x] 12.3 Add `atime: AtimeMode` field to all `MountSpecConfig` variants (with `#[serde(default)]` so default is `AtimeMode::Default`)
- [x] 12.4 Apply `#[serde(rename_all = "camelCase")]` to the `MountSpecConfig` enum so all fields are camelCase in YAML (readOnly, noExec, noSuid, noDev, dirSync, mandatoryLocks, posixAcl, atime remains same)
- [x] 12.5 Removed the old `flags: Option<Vec<String>>` field entirely from all `MountSpecConfig` variants
- [x] 12.6 Updated `impl From<MountSpecConfig> for ocelot_bootstrap::MountSpec` to build `MsFlags` by OR-ing bits from boolean fields
- [x] 12.7 Updated template YAML files (`templates/bootstrap/*.yaml`) to demonstrate new boolean flag fields with comments explaining defaults and showing camelCase keys
- [x] 12.8 Added unit tests for `AtimeMode` deserialization and `MsFlags` conversion covering all boolean combinations and atime variants
- [x] 12.9 Ran `cargo clippy-all` and fixed all warnings (added appropriate `#[expect]` attributes)
- [x] 12.10 Ran `cargo test` in `ocelot` crate; all config parsing tests pass
- [x] 12.11 Updated documentation in templates and inline doc comment for build_flags; no README changes needed
