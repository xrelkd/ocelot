## 1. Create Shutdown Module

- [x] 1.1 Create `crates/bootstrap/src/shutdown.rs` with `pub fn shutdown() -> Result<(), Error>`
- [x] 1.2 Implement shutdown using `nix::sys::reboot::reboot(RB_AUTOBOOT)`
- [x] 1.3 Add `ShutdownSnafu` error variant to `crates/bootstrap/src/error.rs`
- [x] 1.4 Add `pub mod shutdown;` and `pub use self::shutdown::shutdown;` to `lib.rs`

## 2. Integrate with CLI

- [x] 2.1 Import `ocelot_bootstrap::shutdown` in `ocelot/src/cli/bootstrap.rs`
- [x] 2.2 Call `shutdown()` after `execute_shell` returns (line 27)
- [x] 2.3 Call `shutdown()` after `execute_supervise` returns (line 30)
- [x] 2.4 Handle shutdown errors appropriately in CLI

## 3. Verify Implementation

- [x] 3.1 Run `cargo clippy --workspace --all-targets` - no warnings
- [x] 3.2 Run `cargo build` - compiles successfully
- [x] 3.3 Run `cargo nextest run` - tests pass
