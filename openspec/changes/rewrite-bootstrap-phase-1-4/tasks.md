## 1. Remove Deprecated Configuration and Create Independent Files

- [ ] 1.1 Remove all deprecated configuration structs and enums from `crates/bootstrap/src/config.rs` and related files
- [ ] 1.2 Create new directory `ocelot/src/config/bootstrap/` for independent configuration files
- [ ] 1.3 Create `ocelot/src/config/bootstrap/bootstrap.rs` with main BootstrapConfig struct
- [ ] 1.4 Create `ocelot/src/config/bootstrap/pre_switch.rs` with PreSwitchConfig struct
- [ ] 1.5 Create `ocelot/src/config/bootstrap/switch_root.rs` with SwitchRootConfig struct
- [ ] 1.6 Create `ocelot/src/config/bootstrap/post_switch.rs` with PostSwitchConfig struct
- [ ] 1.7 Create `ocelot/srcconfig/bootstrap/mount_spec.rs` with MountSpecConfig and MountSpec types
- [ ] 1.8 Create `ocelot/src/config/bootstrap/modules_config.rs` with ModulesConfig types
- [ ] 1.9 Create `ocelot/src/config/bootstrap/network_config.rs` with NetworkConfig types (mark unsupported fields with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 1.10 Create `ocelot/src/config/bootstrap/hook_spec.rs` with HookSpecConfig types
- [ ] 1.11 Create `ocelot/src/config/bootstrap/sysctl_config.rs` with SysctlConfig types
- [ ] 1.12 Create `ocelot/src/config/bootstrap/tmpfile_config.rs` with TmpfileConfig types (mark user/group with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 1.13 Create `ocelot/src/config/bootstrap/security_config.rs` with SecurityConfig types (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 1.14 Create `ocelot/src/config/bootstrap/clock_config.rs` with ClockConfig types (mark ntp with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 1.15 Create `ocelot/src/config/bootstrap/handoff_config.rs` with HandoffConfig types
- [ ] 1.16 Create `ocelot/src/config/bootstrap/shutdown_config.rs` with ShutdownConfig types
- [ ] 1.17 Implement `From` trait for each config type to convert to runtime equivalents
- [ ] 1.18 Update `ocelot/src/config/bootstrap/bootstrap.rs` to re-export all submodules
- [ ] 1.19 Update `ocelot/src/config/bootstrap.rs` to use the new independent files instead of inline definitions
- [ ] 1.20 Verify no deprecated configuration remains in the codebase

## 2. Enhanced Validation with Dependency Detection

- [ ] 2.1 Import module dependency detection functions in `ocelot/src/config/bootstrap.rs`
- [ ] 2.2 Import process dependency detection functions in `ocelot/src/config/bootstrap.rs`
- [ ] 2.3 Update `BootstrapConfig::validate()` to call module dependency validation when dep_file_path is provided
- [ ] 2.4 Update `BootstrapConfig::validate()` to call process dependency validation for handoff processes
- [ ] 2.5 Add new error variants for module dependency failures and process dependency failures
- [ ] 2.6 Ensure validation returns early on first failure encountered
- [ ] 2.7 Maintain existing validation checks (environment variables, mode exclusivity)
- [ ] 2.8 Test validation with valid module dependencies
- [ ] 2.9 Test validation with invalid module dependencies (circular deps)
- [ ] 2.10 Test validation with valid process dependencies
- [ ] 2.11 Test validation with invalid process dependencies (undefined process)
- [ ] 2.12 Verify `cargo clippy-all` passes with no warnings

## 3. Serialization Layer: New BootstrapConfig Structure

- [ ] 3.1 Define `PreSwitchConfig` struct in `ocelot/src/config/bootstrap/pre_switch.rs` with fields: modules, network, mounts, hooks, environment, symlinks, sysctl, tmpfiles, security, clock (all with `#[serde(default)]`)
- [ ] 3.2 Define `PostSwitchConfig` struct in `post_switch.rs` with same fields as PreSwitchConfig plus handoff and shutdown
- [ ] 3.3 Define `SwitchRootConfig` struct in `switch_root.rs` with fields: method, oldRootDir, cleanupOldRoot, moveSpecial
- [ ] 3.4 Redefine `BootstrapConfig` in `bootstrap.rs` to have `pre_switch`, `switch_root`, `post_switch` fields; remove legacy flat fields
- [ ] 3.5 Update `BootstrapConfig::validate()` to validate the new three-tier structure
- [ ] 3.6 Update `BootstrapConfig::to_bootstrap_config()` to convert all three phases using From implementations
- [ ] 3.7 Define `MountSpecConfig` enum (serde-tagged) with variants for Device, VirtiofsTag, NinePTag, Virtual, Nfs, Overlay
- [ ] 3.8 Define `MountFailurePolicy` enum with Warn, Abort, Retry variants
- [ ] 3.9 Define `ModulesConfig` enum (List/Scan modes)
- [ ] 3.10 Define `NetworkConfig` struct with dhcp/static modes, interfaces, firewall (mark unused fields with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 3.11 Define `HookSpecConfig` struct with name, command, arguments, timeout, onFailure
- [ ] 3.12 Define `SysctlConfig` as type alias for `HashMap<String, serde_yaml::Value>`
- [ ] 3.13 Define `TmpfileConfig` struct with path, mode, type, user, group (mark user/group with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 3.14 Define `SecurityConfig` struct with selinux and apparmor sub-configs (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 3.15 Define `ClockConfig` struct with rtcSync and optional ntp (mark ntp with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 3.16 Define `HandoffConfig` struct with mode, bootScript, supervise, shell
- [ ] 3.17 Define `ShutdownConfig` struct with timeout, sync, umountAll
- [ ] 3.18 Implement all `From` implementations for config types to their runtime counterparts
- [ ] 3.19 Implement string-to-MsFlags conversion for mount flag parsing
- [ ] 3.20 Verify `cargo clippy-all` passes with no warnings (all unused items have `#[expect(dead_code, reason = "unsupported yet"]`)

## 4. Runtime Layer: New Config Structure

- [ ] 4.1 Define `PreSwitchPhase` struct in `crates/bootstrap/src/config.rs` mirroring PreSwitchConfig
- [ ] 4.2 Define `PostSwitchPhase` struct mirroring PostSwitchConfig
- [ ] 4.3 Define `SwitchRootPhase` struct mirroring SwitchRootConfig
- [ ] 4.4 Redefine `Config` to have `pre_switch`, `switch_root`, `post_switch` fields; remove legacy flat fields
- [ ] 4.5 Update `Config::default()` for new structure
- [ ] 4.6 Define `MountSpec` struct with source, target, fstype, flags, options, overlay, on_failure
- [ ] 4.7 Define `MountSource` enum with Device, VirtiofsTag, NinePTag, Virtual, Nfs, Overlay variants (mark unused with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 4.8 Define `MountFailurePolicy` enum (Abort, Warn, Retry) in runtime layer
- [ ] 4.9 Define `OverlaySpec` struct for overlay filesystem configuration
- [ ] 4.10 Update all `From` implementations from serialization types to runtime types
- [ ] 4.11 Remove legacy `ModuleConfig` struct or verify it's removed
- [ ] 4.12 Update re-exports in `lib.rs` for new public types
- [ ] 4.13 Update unit tests in `config.rs` for new structures
- [ ] 4.14 Verify `cargo clippy-all` passes with no warnings

## 5. Mount System: MountSpec and Infrastructure

- [ ] 5.1 Add `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` at start of `mount_virtual_filesystems()`
- [ ] 5.2 Add `/dev/pts` (devpts) mount to `mount_virtual_filesystems()`
- [ ] 5.3 Add `/dev/shm` (tmpfs) mount to `mount_virtual_filesystems()`
- [ ] 5.4 Add `/tmp` (tmpfs) mount to `mount_virtual_filesystems()`
- [ ] 5.5 Add `/run/lock` directory creation to `mount_virtual_filesystems()`
- [ ] 5.6 Modify `mount_move_special()` to accept `extra_targets: &[PathBuf]` parameter
- [ ] 5.7 Add `/dev/pts` and `/dev/shm` to the standard move list in `mount_move_special()`
- [ ] 5.8 Implement extra target iteration in `mount_move_special()` after standard moves
- [ ] 5.9 Add `PivotRoot` error variant to `error.rs`
- [ ] 5.10 Verify `cargo clippy-all` passes with no warnings

## 6. switch_root: Split and pivot_root Implementation with Fully Qualified Names

- [ ] 6.1 Create `switch_root::only(config)` function that performs pivot_root without exec
- [ ] 6.2 Implement `nix::mount::mount(None, "/", None, nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE, None)` in `switch_root::only()`
- [ ] 6.3 Implement pivot_root flow: mkdir oldroot → nix::unistd::pivot_root → nix::unistd::chdir("/") → nix::mount::umount2 oldroot → nix::unistd::rmdir oldroot
- [ ] 6.4 Add chroot fallback path when `config.method == chroot` using nix::unistd::chdir and nix::unistd::chroot
- [ ] 6.5 Create `switch_root::exec_supervise(orchestrator_config)` function calling `ocelot_supervise::execute()`
- [ ] 6.6 Create `switch_root::exec_shell(console_device, shell_config)` function for shell handoff
- [ ] 6.7 Remove legacy `switch_root()` and `switch_root_shell()` functions entirely from codebase
- [ ] 6.8 Ensure ALL nix function calls use fully qualified names (no `use nix::xxx;` statements)
- [ ] 6.9 Verify `cargo clippy-all` passes with no warnings

## 7. Phase Module: Create Structure and Stub Functions

- [ ] 7.1 Create `crates/bootstrap/src/phase/mod.rs` with module declarations
- [ ] 7.2 Create `crates/bootstrap/src/phase/clock.rs` with `pre()` (RTC sync) and `post()` stub (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 7.3 Create `crates/bootstrap/src/phase/sysctl.rs` with `pre()` and `post()` (write to `/proc/sys/` using fully qualified nix names)
- [ ] 7.4 Create `crates/bootstrap/src/phase/tmpfiles.rs` with `pre()` and `post()` (create dirs/files with mode)
- [ ] 7.5 Create `crates/bootstrap/src/phase/symlinks.rs` with `pre()` and `post()` (create symlinks)
- [ ] 7.6 Create `crates/bootstrap/src/phase/environment.rs` with `pre()` and `post()` (set env vars via `std::env::set_var`)
- [ ] 7.7 Create `crates/bootstrap/src/phase/modules.rs` with `pre()` (call existing `modules::load_modules`) and `post()` stub (mark with `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 7.8 Create `crates/bootstrap/src/phase/network.rs` with `pre()` and `post()` stubs (both marked `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 7.9 Create `crates/bootstrap/src/phase/mounts.rs` with `pre()` (mount at /newroot+target, return Vec<PathBuf>) and `post()` (mount at target directly)
- [ ] 7.10 Create `crates/bootstrap/src/phase/hooks.rs` with `pre()` and `post()` (execute hook commands with timeout and onFailure handling)
- [ ] 7.11 Create `crates/bootstrap/src/phase/security.rs` with `post()` stub (marked `#[expect(dead_code, reason = "unsupported yet"]`)
- [ ] 7.12 Create `crates/bootstrap/src/phase/handoff.rs` with `execute()` function (boot script + supervise/shell handoff)
- [ ] 7.13 Update `mod.rs` to re-export all phase functions
- [ ] 7.14 Verify `cargo clippy-all` passes with no warnings

## 8. Execution Flow: Rewrite execute_supervise and execute_shell

- [ ] 8.1 Rewrite `execute_supervise()` to follow phased order: virtual_filesystems → clock::pre → sysctl::pre → tmpfiles::pre → symlinks::pre → environment::pre → modules::pre → network::pre → mounts::pre → hooks::pre
- [ ] 8.2 Add `switch_root::only()` call after preSwitch phases
- [ ] 8.3 Add postSwitch phase calls: hooks::post → symlinks::post → environment::post → tmpfiles::post → sysctl::post → mounts::post → network::post → modules::post → security::post → clock::post
- [ ] 8.4 Add boot script execution after postSwitch, before handoff
- [ ] 8.5 Add `switch_root::exec_supervise()` as final handoff
- [ ] 8.6 Rewrite `execute_shell()` with same phased order, ending with `switch_root::exec_shell()`
- [ ] 8.7 Remove old direct calls to `mount_extra_virtiofs`, `create_symlinks`, `load_modules` from lib.rs
- [ ] 8.8 Update module declarations in lib.rs to include `phase` module
- [ ] 8.9 Update doc comments for `execute_supervise` and `execute_shell` to reflect new flow
- [ ] 8.10 Ensure ALL nix function calls in lib.rs use fully qualified names
- [ ] 8.11 Verify `cargo clippy-all` passes with no warnings

## 9. Config Templates: Update YAML Examples

- [ ] 9.1 Rewrite `ocelot/src/config/templates/bootstrap/shell.yaml` with new preSwitch/switchRoot/postSwitch structure
- [ ] 9.2 Rewrite `ocelot/src/config/templates/bootstrap/supervise.yaml` with new structure and all subsystem examples
- [ ] 9.3 Add comments explaining each subsystem's preSwitch vs postSwitch purpose
- [ ] 9.4 Verify templates parse correctly with `BootstrapConfig::load()`

## 10. Legacy Code Cleanup

- [ ] 10.1 Search for and remove any remaining deprecated code throughout the bootstrap crate
- [ ] 10.2 Remove any unused imports or dead code that was missed in previous steps
- [ ] 10.3 Verify all `#[expect(dead_code, reason = "unsupported yet"]` annotations are correct and necessary
- [ ] 10.4 Ensure no commented-out code or TODOs remain that should be addressed
- [ ] 10.5 Run final verification that the codebase is clean of deprecated artifacts

## 11. Integration Verification

- [ ] 11.1 Run `cargo clippy-all` and fix all warnings
- [ ] 11.2 Run `cargo fmt --all --check` and fix formatting
- [ ] 11.3 Run `cargo nextest-all` and fix all test failures
- [ ] 11.4 Run `cargo doc-all` and verify documentation generates without warnings
- [ ] 11.5 Verify `cargo build --release` succeeds for bootstrap crate
- [ ] 11.6 Verify `cargo build --release` succeeds for ocelot binary
