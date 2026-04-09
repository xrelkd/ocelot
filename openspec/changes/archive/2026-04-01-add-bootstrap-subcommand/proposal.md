# Add bootstrap Subcommand

## What

Add a new `bootstrap` subcommand (with `boot` alias) to ocelot that acts as an initramfs init system, designed primarily for QEMU virtual machines. This includes:

- A new `crates/bootstrap` crate that handles early boot initialization
- Integration with the existing `supervise` crate for process management
- A new CLI subcommand `ocelot bootstrap` (alias: `boot`) with YAML configuration
- Support for QEMU-specific features (virtiofs, virtio-blk, 9p)

## Why

Ocelot currently provides process supervision (`supervise`, `entry`, `idle`) but lacks boot-time initialization capabilities. Adding bootstrap support would:

1. **Enable QEMU VM boot**: Allow ocelot to serve as the PID 1 init process in QEMU virtual machines, handling root filesystem mounting and process supervision in a single binary
2. **Unify boot and runtime management**: After mounting the root filesystem, seamlessly transition to the existing supervise orchestrator for managing application processes
3. **Reduce infrastructure complexity**: Replace separate initrd + init systems with a single ocelot binary that handles both phases
4. **Leverage existing investment**: The supervise crate already provides robust process management, dependency ordering, health probing, and log rotation - bootstrap just needs to set up the environment for it

## Scope

### In Scope

- New `crates/bootstrap` crate with QEMU-focused boot support
- YAML configuration combining bootstrap-specific options with supervise configuration
- CLI subcommand `ocelot bootstrap` (alias: `boot`) with `--file <config>` option
- Root filesystem mounting (virtio-blk, virtiofs, 9p)
- Virtual filesystem setup (proc, sysfs, devtmpfs)
- Kernel module loading via `finit_module()`
- Switch root and handoff to `supervise::execute()`
- Basic error handling with optional debug shell on failure

### Out of Scope (Future)

- Physical hardware detection
- LVM/encryption support
- Network boot (NFS root)
- initramfs image building tools
- Cross-compilation infrastructure

## Implementation Status

**Completed:**

- `crates/bootstrap` crate with all modules (config, cmdline, console, modules, mount, switch_root, error)
- CLI integration with `bootstrap` subcommand and `boot` alias
- Full config parsing with `BootstrapConfig` supporting virtiofs/block/9p root types
- Kernel module loading via `finit_module()` syscall
- Virtual filesystem mounting (proc, sysfs, devtmpfs, tmpfs)
- Overlay filesystem support
- Switch root implementation (chroot + exec)
- Full config conversion from bootstrap format to `ocelot_supervise::OrchestratorConfig`
- All 175 existing tests pass, clippy clean, formatting clean
