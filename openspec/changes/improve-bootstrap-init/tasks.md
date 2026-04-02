## 1. Console Setup Enhancement

- [x] 1.1 Add TIOCSCTTY ioctl call in `crates/bootstrap/src/console.rs` after dup2 operations
- [x] 1.2 Use libc::ioctl with TIOCSCTTY constant (0x5409) to set controlling terminal
- [x] 1.3 Add unit test for console setup (mock file descriptor)

## 2. Overlay Directory Isolation

- [x] 2.1 Modify `mount_overlay()` in `crates/bootstrap/src/mount.rs` to accept source identifier parameter
- [x] 2.2 Implement `overlay_base()` function that sanitizes source name and returns `/run/overlayfs/{safe_name}/`
- [x] 2.3 Update `mount_root()` to pass source identifier when calling mount_overlay
- [x] 2.4 Update `lib.rs` to pass config.root.source() to mount_overlay

## 3. Shell Configuration in Bootstrap Crate

- [x] 3.1 Add `ShellConfig` struct to `crates/bootstrap/src/config.rs` with `program: String` and `args: Vec<String>`
- [x] 3.2 Export `ShellConfig` from `lib.rs`

## 4. Shell Execution Function

- [x] 4.1 Create new function `execute_shell(config: &Config, shell_config: &ShellConfig) -> Result<(), Error>` in `lib.rs`
- [x] 4.2 Implement console setup, VFS mounting, root mounting, switch_root in execute_shell
- [x] 4.3 After switch_root, spawn shell process with controlling terminal
- [x] 4.4 Wait for shell exit, then return (CLI can handle shutdown)

## 5. Shell Config in CLI

- [x] 5.1 Add `ShellConfig` struct to `ocelot/src/config/bootstrap.rs` with serde
- [x] 5.2 Add `shell: Option<ShellConfig>` to `BootstrapConfig`
- [x] 5.3 Add validation: if `shell` is set, `processes` must be empty
- [x] 5.4 Add `to_shell_config()` method to convert to `ocelot_bootstrap::ShellConfig`

## 6. CLI Mode Selection

- [x] 6.1 Update `ocelot/src/cli/bootstrap.rs` to check if shell mode is configured
- [x] 6.2 If shell mode: call `ocelot_bootstrap::execute_shell(&bootstrap_config, &shell_config)`
- [x] 6.3 If supervise mode (default): call `ocelot_bootstrap::execute(&bootstrap_config, orchestrator)`

## 7. Testing and Verification

- [x] 7.1 Run `cargo clippy --workspace --all-targets` to verify no lint errors
- [x] 7.2 Run `cargo test` in `crates/bootstrap` to verify unit tests pass
- [x] 7.3 Run `cargo build -p ocelot` to verify full binary builds
