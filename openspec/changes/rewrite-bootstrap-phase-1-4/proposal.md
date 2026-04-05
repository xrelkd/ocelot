## Why

The current bootstrap flow has critical structural problems: boot scripts execute _after_ supervise/shell exits instead of before handoff, `switch_root` uses `chroot` instead of `pivot_root` (leaving initramfs in memory), and the flat configuration cannot express which operations must happen before vs. after `switch_root`. This makes it impossible to support network roots, proper mount isolation, or phased system initialization.

Additionally, the current configuration validation does not properly validate module dependencies and process dependencies, and deprecated configuration structures remain in the codebase causing confusion.

## What Changes

- **Remove deprecated configuration structs and enums** from bootstrap configuration
- **Create new independent configuration files** in `ocelot/src/config/` for better organization
- **Restructure `BootstrapConfig`** from flat fields to `preSwitch` / `switchRoot` / `postSwitch` three-tier configuration in a dedicated file
- **Restructure runtime `Config`** to match: `PreSwitchPhase` / `SwitchRootPhase` / `PostSwitchPhase`
- **Introduce `MountSpec` / `MountSource`** abstractions replacing hardcoded virtiofs/block/9p paths
- **Add new subsystem types**: `ModulesConfig`, `NetworkConfig`, `HookSpecConfig`, `SysctlConfig`, `TmpfileConfig`, `SecurityConfig`, `ClockConfig`, `HandoffConfig`, `ShutdownConfig`
- **Enhance validation** to apply module dependency detection and process dependency detection during config validation
- **Create `phase/` module** with `pre()` / `post()` functions per subsystem (clock, sysctl, tmpfiles, symlinks, environment, modules, network, mounts, hooks, security, handoff)
- **Rewrite `execute_supervise` / `execute_shell`** to follow phased execution order with proper timing
- **Split `switch_root`** into `only()` (pivot_root only, no exec) and separate handoff functions
- **Add mount flag support** (`MS_RDONLY`, `MS_NOEXEC`, `MS_NOSUID`, etc.) to `MountSpec`
- **Add mount namespace isolation** (`MS_REC | MS_PRIVATE`) before switch_root operations
- **Add `/dev/pts`, `/dev/shm`, `/tmp`** to virtual filesystem mounts
- **Use fully qualified names** for nix crate functions (e.g., `nix::mount::mount()` instead of `mount::mount()`)
- **Remove phase X descriptions** from comments and replace with clear unsupported yet reasons
- **Update config templates** (shell.yaml, supervise.yaml) to new structure

**BREAKING**: All existing bootstrap YAML configs must be rewritten to the new `preSwitch` / `switchRoot` / `postSwitch` structure.

## Capabilities

### New Capabilities

- `bootstrap-independent-config`: Independent configuration files in ocelot/src/config/
- `bootstrap-phase-config`: Pre/post switch phase configuration structure for all subsystems
- `bootstrap-enhanced-validation`: Module dependency detection and process dependency detection in validation
- `bootstrap-mount-spec`: Generic mount specification with source types, flags, and failure policies
- `bootstrap-phase-execution`: Phased execution engine with pre()/post() per subsystem
- `bootstrap-switch-root`: Proper pivot_root-based root switching with namespace isolation
- `bootstrap-subsystem-types`: Configuration types for modules, network, hooks, sysctl, tmpfiles, security, clock, handoff, shutdown
- `bootstrap-fully-qualified-nix`: Fully qualified nix crate function usage

### Modified Capabilities

- None (all existing bootstrap behavior is being restructured, not incrementally modified)

## Impact

- **`ocelot/src/config/bootstrap.rs`**: New independent configuration file with three-tier structure
- **`crates/bootstrap/src/config.rs`**: Complete rewrite of runtime config layer
- **`crates/bootstrap/src/lib.rs`**: New phased execution flow using fully qualified nix names
- **`crates/bootstrap/src/switch_root.rs`**: Split into `only()` + handoff, pivot_root implementation with fully qualified names
- **`crates/bootstrap/src/mount.rs`**: MountSpec abstraction, namespace isolation, additional virtual filesystems
- **`crates/bootstrap/src/phase/`**: New module directory with subsystem-specific pre/post functions
- **`crates/bootstrap/src/error.rs`**: New error variants for pivot_root, mount flags, etc.
- **`ocelot/src/config/templates/bootstrap/*.yaml`**: Template rewrites to new structure
- **`ocelot/src/cli/bootstrap.rs`**: May need updates for new config paths
