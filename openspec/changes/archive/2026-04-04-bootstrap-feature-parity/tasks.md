## 1. Library Config Types (crates/bootstrap/src/config.rs)

- [x] 1.1 Add `VirtiofsMount` struct with `tag`, `path`, `with_overlay`, and `options` fields (no serde derives)
- [x] 1.2 Add `SymlinkSpec` struct with `source` and `target` fields (no serde derives)
- [x] 1.3 Add `BootScriptConfig` struct with `command`, `args`, `on_failure`, and `working_directory` fields (no serde derives)
- [x] 1.4 Add `OnFailurePolicy` enum (`Warn`, `Abort`) (no serde derives)
- [x] 1.5 Convert `ModulesConfig` from flat struct to enum: `List { dir: Option<String>, names: Vec<String> }` and `Scan { dir: String }` (no serde derives)
- [x] 1.6 Add `extra_virtiofs_mounts`, `symlinks`, `boot_script` fields to `Config` struct; update `modules` field type to new `ModulesConfig` enum
- [x] 1.7 Implement `Default` for all new config structs with sensible defaults
- [x] 1.8 Document `ModulesConfig::List` behavior when `dir` is `None` (defaults to `/lib/modules`)

## 2. CLI Config Types (ocelot/src/config/bootstrap.rs)

- [x] 2.1 Add `VirtiofsMountConfig` struct with `Deserialize` for `extra_virtiofs_mounts` YAML field
- [x] 2.2 Add `SymlinkConfig` struct with `Deserialize` for `symlinks` YAML field
- [x] 2.3 Add `BootScriptConfig` struct with `Deserialize` for `boot_script` YAML field (with `on_failure` defaulting to `warn`)
- [x] 2.4 Add `OnFailurePolicy` enum with `Deserialize` (`warn` / `abort`)
- [x] 2.5 Convert `ModulesConfig` to serde-compatible enum with `#[serde(tag = "mode")]` for `list`/`scan` variants
- [x] 2.6 Add `extra_virtiofs_mounts`, `symlinks`, `boot_script` fields to `BootstrapConfig` with `#[serde(default)]`
- [x] 2.7 Add `From` impl: CLI `ModulesConfig` → library `ModulesConfig`
- [x] 2.8 Add `From` impl: CLI `VirtiofsMountConfig` → library `VirtiofsMount`
- [x] 2.9 Add `From` impl: CLI `SymlinkConfig` → library `SymlinkSpec`
- [x] 2.10 Add `From` impl: CLI `BootScriptConfig` → library `BootScriptConfig`
- [x] 2.11 Update `BootstrapConfig::to_bootstrap_config()` to map all new fields

## 3. Virtiofs Support Detection

- [x] 3.1 Create `check_virtiofs_support()` function that reads `/proc/filesystems` and checks for `virtiofs` entry
- [x] 3.2 Add `VirtiofsNotSupported` variant to the error enum with descriptive message
- [x] 3.3 Add unit tests for virtiofs support detection

## 4. Extra Virtiofs Mounts

- [x] 4.1 Create `mount_extra_virtiofs()` function that iterates over `extra_virtiofs_mounts` config
- [x] 4.2 Implement `mount_virtiofs_share()` for single share with tag, path, and options
- [x] 4.3 Implement `mount_overlay_for_share()` for per-share overlayfs with isolated `/run/overlayfs/{tag}/` directories
- [x] 4.4 Add `ensure_dir_all()` helper for recursive directory creation with mode 0755
- [x] 4.5 Integrate extra mounts into `execute_supervise()` after root mount and before symlinks
- [x] 4.6 Integrate extra mounts into `execute_shell()` after root mount and before symlinks
- [x] 4.7 Add error handling: log warnings on individual mount failures, continue boot flow
- [x] 4.8 Add unit tests for overlay directory path generation

## 5. Module Scan Mode

- [x] 5.1 Refactor `load_modules()` in `modules.rs` to dispatch on `ModulesConfig` enum
- [x] 5.2 Implement `scan_and_load_modules()` that reads directory, filters `.ko`/`.ko.xz`/`.ko.gz` files, and loads each
- [x] 5.3 Add loading summary logging (loaded count, failed count, total count) for scan mode
- [x] 5.4 Handle missing directory gracefully in scan mode (skip with info log)
- [x] 5.5 Update existing list-mode tests to work with new `ModulesConfig::List` variant
- [x] 5.6 Add tests for scan mode file filtering (correct extensions, skip non-modules)

## 6. Symlink Creation

- [x] 6.1 Create `symlinks.rs` module with `create_symlinks()` function
- [x] 6.2 Implement `create_symlink()` for single symlink with parent directory creation via `ensure_dir_all()`
- [x] 6.3 Add warning log when symlink target does not exist (symlink still created)
- [x] 6.4 Integrate symlink creation into `execute_supervise()` after extra mounts and before boot script
- [x] 6.5 Integrate symlink creation into `execute_shell()` after extra mounts and before boot script
- [x] 6.6 Add unit tests for symlink creation

## 7. Boot Script Execution

- [x] 7.1 Create `script.rs` module with `execute_boot_script()` function
- [x] 7.2 Implement script execution using `entry::execute` (zombie reaping, signal forwarding, timeout support) with command, args, working directory, and inherited environment
- [x] 7.3 Add `on_failure` policy handling: `Warn` logs and continues, `Abort` returns error
- [x] 7.4 Integrate boot script into `execute_supervise()` after switch_root and before supervise handoff
- [x] 7.5 Integrate boot script into `execute_shell()` after switch_root and before shell spawn
- [x] 7.6 Add unit tests for boot script failure policy behavior

## 8. Boot Flow Integration

- [x] 8.1 Update `execute_supervise()` to call virtiofs support check before any virtiofs mounts
- [x] 8.2 Update `execute_supervise()` boot flow order: console → VFS → modules → root → overlay → extra mounts → symlinks → env → working_dir → switch_root → boot_script → supervise
- [x] 8.3 Update `execute_shell()` boot flow order: console → VFS → modules → root → overlay → extra mounts → symlinks → env → working_dir → switch_root → boot_script → shell
- [x] 8.4 Ensure all new stages are skipped gracefully when not configured (empty lists, None options)
- [x] 8.5 Add tracing info logs for each new boot stage entry

## 9. Testing and Validation

- [x] 9.1 Run `cargo fmt --all --check` to verify formatting
- [x] 9.2 Run `cargo clippy-all` to verify no lint warnings
- [x] 9.3 Run `cargo nextest-all` to verify all tests pass
- [x] 9.4 Run `cargo doc-all` to verify documentation generates without warnings
- [x] 9.5 Add integration test config YAML examples demonstrating all new features
- [x] 9.6 Update `config-template` CLI output to include new config fields with comments
