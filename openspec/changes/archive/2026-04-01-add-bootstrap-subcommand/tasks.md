## 1. Workspace and Crate Setup

- [x] 1.1 Create `crates/bootstrap/` directory with `Cargo.toml` and `src/lib.rs`
- [x] 1.2 Add `ocelot-bootstrap` to workspace members in root `Cargo.toml`
- [x] 1.3 Add `ocelot-bootstrap = { path = "./crates/bootstrap" }` to workspace dependencies
- [x] 1.4 Add `ocelot-bootstrap` dependency to `ocelot` binary crate
- [x] 1.5 Add `ocelot-supervise` as a dependency of `ocelot-bootstrap`
- [x] 1.6 Verify `cargo check -p ocelot-bootstrap` passes with empty lib

## 2. Config Module

- [x] 2.1 Create `crates/bootstrap/src/config.rs` with `BootstrapConfig` struct
- [x] 2.2 Define `RootConfig` enum/struct supporting virtiofs, block, and 9p types
- [x] 2.3 Define `ModuleConfig` struct with dir and list fields
- [x] 2.4 Define `OnFailureConfig` struct with optional shell path
- [x] 2.5 Embed supervise config fields directly in `BootstrapConfig`
- [x] 2.6 Implement `BootstrapConfig::load(path)` using serde_yaml
- [x] 2.7 Implement `BootstrapConfig::validate()` with field-level validation
- [x] 2.8 Implement `BootstrapConfig::template_basic()` returning YAML string
- [x] 2.9 Add unit tests for config parsing (valid, minimal, invalid cases)

## 3. Cmdline Module

- [x] 3.1 Create `crates/bootstrap/src/cmdline.rs`
- [x] 3.2 Implement `read_cmdline()` reading `/proc/cmdline`
- [x] 3.3 Implement `parse_cmdline()` extracting ocelot-specific parameters
- [x] 3.4 Support cmdline override of config values (e.g., `ocelot.root=/dev/vda2`)
- [x] 3.5 Add unit tests for cmdline parsing

## 4. Console Module

- [x] 4.1 Create `crates/bootstrap/src/console.rs`
- [x] 4.2 Implement `setup(device)` opening `/dev/<device>` and dup2 to stdin/stdout/stderr
- [x] 4.3 Implement `setsid()` for session leadership
- [x] 4.4 Add SAFETY comments for unsafe blocks
- [x] 4.5 Add unit tests for console setup (mocked)

## 5. Modules Loader

- [x] 5.1 Create `crates/bootstrap/src/modules.rs`
- [x] 5.2 Implement `load_module(path)` using `nix::kmod::finit_module()`
- [x] 5.3 Implement `load_modules(config)` iterating over module list
- [x] 5.4 Handle module load failures gracefully (log warning, continue)
- [x] 5.5 Add SAFETY comments for finit_module syscall
- [x] 5.6 Add unit tests (empty list test)

## 6. Mount Module

- [x] 6.1 Create `crates/bootstrap/src/mount.rs`
- [x] 6.2 Implement `mount_virtual_filesystems()` mounting proc, sysfs, devtmpfs, tmpfs
- [x] 6.3 Implement `mount_root(config)` for virtiofs, block, and 9p backends
- [x] 6.4 Implement `wait_for_device(path, timeout)` polling loop for block devices
- [x] 6.5 Implement `mount_overlay(root_config)` setting up upper/work dirs and overlay mount
- [x] 6.6 Implement `mount_move_special()` moving /proc, /sys, /dev, /run to newroot via MS_MOVE
- [x] 6.7 Add error types for mount failures with context
- [x] 6.8 Add unit/integration tests for mount operations

## 7. Switch Root Module

- [x] 7.1 Create `crates/bootstrap/src/switch_root.rs`
- [x] 7.2 Implement `switch_root(config)` performing chdir + chroot + exec
- [x] 7.3 Implement handoff to `supervise::execute()` after switch_root
- [x] 7.4 Validate supervise config before switch_root
- [x] 7.5 Add SAFETY comments for exec syscall
- [x] 7.6 Add unit tests (mocked exec)

## 8. Error Module

- [x] 8.1 Create `crates/bootstrap/src/error.rs` with snafu-based `Error` enum
- [x] 8.2 Define error variants: ConfigLoad, ConfigValidate, ConsoleSetup, ModuleLoad, Mount, SwitchRoot, SuperviseHandoff
- [x] 8.3 Implement `Display` and `Debug` via snafu derives
- [x] 8.4 Add error context for each operation

## 9. Main Execute Function

- [x] 9.1 Implement `execute(config: &BootstrapConfig) -> Result<(), Error>` in `lib.rs`
- [x] 9.2 Wire up boot sequence: console → virtual fs → modules → root mount → overlay → switch_root → supervise
- [x] 9.3 Implement error recovery: spawn debug shell or loop on failure
- [x] 9.4 Add tracing log points at each boot stage
- [x] 9.5 Verify PID 1 check (warn if not PID 1)

## 10. CLI Integration

- [x] 10.1 Add `Bootstrap` variant to `Commands` enum in `cli/mod.rs` with `visible_aliases = ["boot"]`
- [x] 10.2 Add `--file` and `--log-level` arguments to bootstrap subcommand
- [x] 10.3 Add `bootstrap config-template` sub-subcommand
- [x] 10.4 Add `bootstrap validate` sub-subcommand with human/json output
- [x] 10.5 Implement bootstrap handler in `cli/mod.rs` calling `ocelot_bootstrap::execute()`
- [x] 10.6 Update `Cli::run()` match arm for Bootstrap command
- [x] 10.7 Verify `ocelot bootstrap --help` and `ocelot boot --help` display correct help text

## 11. Testing and Verification

- [x] 11.1 Add unit tests for config parsing and validation
- [x] 11.2 Add unit tests for cmdline parsing
- [x] 11.3 Add integration test for mount operations (skip if not root)
- [x] 11.4 Run `cargo fmt --all --check`
- [x] 11.5 Run `cargo clippy-all`
- [x] 11.6 Run `cargo nextest-all`
- [x] 11.7 Verify `cargo build --release` produces a reasonably-sized binary
