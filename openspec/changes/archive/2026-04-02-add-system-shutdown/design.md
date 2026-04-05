## Context

The ocelot bootstrap system acts as an init system for QEMU VMs. When the bootstrap process completes (shell exits or supervise returns), the VM needs to shut down cleanly. Currently, there is no shutdown mechanism - the system just returns from the execute functions and exits the CLI.

## Goals / Non-Goals

**Goals:**

- Add a `shutdown()` function to `crates/bootstrap/src/shutdown.rs`
- Invoke shutdown in CLI layer after `execute_shell` or `execute_supervise` returns
- Use `nix::sys::reboot::RB_AUTOBOOT` for system power-off
- Return proper error types if shutdown fails

**Non-Goals:**

- Adding shutdown configuration options (timing, signals)
- Graceful service termination before shutdown (handled by supervise)
- Windows or non-Linux platform support

## Decisions

1. **Module location**: Create `shutdown.rs` in `crates/bootstrap/src/` rather than a separate crate
   - Rationale: Bootstrap crate already handles system-level operations (mount, console, switch_root)
   - Alternative: Separate `shutdown` crate - rejected to avoid excessive crate fragmentation

2. **CLI integration**: Call shutdown after execute functions return, not inside them
   - Rationale: Keeps bootstrap crate focused on setup; CLI controls lifecycle
   - Alternative: Shutdown inside execute functions - rejected per user requirement

3. **Error handling**: Use existing snafu error pattern with `ShutdownSnafu` context
   - Rationale: Consistent with existing error handling in the codebase
   - Alternative: Propagate nix errors directly - rejected for consistency

## Risks / Trade-offs

- **Risk**: Shutdown may fail if process lacks privileges
  - **Mitigation**: Bootstrap should run as PID 1 with appropriate capabilities

- **Risk**: Immediate shutdown may not allow supervise to clean up
  - **Mitigation**: This only triggers when execute functions return, which happens after normal operation
