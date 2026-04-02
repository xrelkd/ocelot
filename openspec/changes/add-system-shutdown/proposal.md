## Why

When the bootstrap process completes (shell exits or supervise returns), the system remains in an undefined state rather than performing a clean shutdown. This is problematic for QEMU VMs where a proper power-off is needed to ensure resources are released and the VM terminates cleanly.

## What Changes

- Add a `shutdown()` function to `crates/bootstrap/src/` that performs system power-off using `reboot(RB_AUTOBOOT)`
- Invoke the shutdown function in the CLI after `execute_shell` or `execute_supervise` returns
- The shutdown function is placed in the bootstrap crate but invoked from the CLI layer, keeping separation of concerns

## Capabilities

### New Capabilities

- `system-shutdown`: System power-off capability triggered after bootstrap operations complete

### Modified Capabilities

- None

## Impact

- **New file**: `crates/bootstrap/src/shutdown.rs` - shutdown implementation
- **Modified**: `crates/bootstrap/src/lib.rs` - export shutdown module
- **Modified**: `ocelot/src/cli/bootstrap.rs` - call shutdown after execute_shell/execute_supervise returns
- **Dependencies**: Uses `nix::sys::reboot` for system power-off
