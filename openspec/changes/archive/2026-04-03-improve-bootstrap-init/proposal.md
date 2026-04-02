## Why

The current `ocelot-bootstrap` crate lacks proper controlling terminal setup and has a design limitation in overlay filesystem handling. Without `ioctl(TIOCSCTTY)`, child processes spawned after switch_root lack a controlling terminal, breaking interactive shells and programs that expect terminal control. The single `/run/overlay/` directory for all overlay mounts creates conflicts when multiple mounts use overlay.

Additionally, the system lacks a shell execution mode for debugging VM boot issues. Shell mode and supervise mode should be mutually exclusive in configuration.

## What Changes

- **Console setup**: Add `ioctl(TIOCSCTTY)` call after dup2 to establish the console as the controlling terminal for the session
- **Overlay isolation**: Change overlay directories from `/run/overlay/` to `/run/overlayfs/{source}/` per mount, using the mount source (tag/device) as the identifier
- **Shell execution mode**: Add new `execute_shell()` function and `ShellConfig` to bootstrap crate; CLI decides which mode to invoke
- **Config exclusivity**: Shell mode and supervise mode are mutually exclusive in YAML configuration
- **BREAKING**: Overlay directory path changes from `/run/overlay/` to `/run/overlayfs/{source}/`

## Capabilities

### New Capabilities

- `bootstrap-shell`: Direct shell execution mode for debugging via `execute_shell()` function

### Modified Capabilities

- `bootstrap-boot`:
  - Console setup requirement: add TIOCSCTTY for controlling terminal
  - Overlay directory structure: change to per-mount isolation under `/run/overlayfs/{source}/`

## Impact

- `crates/bootstrap/src/console.rs`: Add `ioctl(TIOCSCTTY)` via libc
- `crates/bootstrap/src/mount.rs`: Restructure overlay directories
- `crates/bootstrap/src/config.rs`: Add `ShellConfig` struct
- `crates/bootstrap/src/lib.rs`: Add `execute_shell()` function
- `crates/bootstrap/src/switch_root.rs`: Support shell execution path
- `ocelot/src/config/bootstrap.rs`: Add shell config option with validation (exclusive with supervise processes)
- `openspec/specs/bootstrap-boot/spec.md`: Update overlay and console requirements
