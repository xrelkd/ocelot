## Context

Ocelot is a Rust process supervisor and init system with multiple crates:

- `ocelot-entry`: Single-process supervisor with signal forwarding and zombie reaping
- `ocelot-idle`: Minimalist PID 1 pause container
- `ocelot-supervise`: Multi-process orchestrator with dependency ordering, health probing, and log rotation
- `ocelot-zombie`: Testing utility for creating zombie processes

All crates target container environments where ocelot runs as PID 1. None handle boot-time initialization (mounting root filesystems, loading kernel modules, switching root).

The project uses `nix` crate (0.31) with `mount` and `process` features already enabled, `snafu` for error handling, `serde_yaml` for configuration, and `clap` for CLI.

## Goals / Non-Goals

**Goals:**

- Enable ocelot to boot QEMU VMs as an initramfs init process
- Create `crates/initrd` that mounts rootfs and hands off to `supervise::execute()`
- Unified YAML config combining initrd boot options with supervise process definitions
- Add `ocelot initrd` CLI subcommand
- Support QEMU-native storage: virtio-blk, virtiofs, 9p
- Keep binary size reasonable for initramfs inclusion

**Non-Goals:**

- Physical hardware detection (target QEMU only)
- LVM, dm-crypt, or complex storage stacks
- NFS root or network boot
- initramfs image building tools (handled by Nix flake)
- Cross-compilation changes (existing Nix infrastructure handles this)

## Decisions

### 1. Use `switch_root` (chroot + exec) instead of `pivot_root`

**Decision:** Use `chroot("/newroot")` + `exec` pattern rather than `pivot_root()`.

**Rationale:**

- Modern kernels use initramfs (tmpfs-based), not initrd (block-device-based)
- `pivot_root` does not work correctly with initramfs — the kernel's rootfs cannot be unmounted
- `switch_root` (move mounts → chroot → exec) is the standard initramfs pattern
- Reference implementations (rsinit, kdf-init) both use chroot-based approaches

**Alternatives considered:**

- `pivot_root`: Only works for initrd (block device ramfs), not initramfs
- Pure `chroot`: Leaves old rootfs mounted, wastes memory

### 2. New `crates/bootstrap` depends on `ocelot-supervise`

**Decision:** The bootstrap crate calls `supervise::execute()` after switch_root, rather than reimplementing process management.

**Rationale:**

- supervise already provides robust process lifecycle, dependency ordering, health probing, log rotation
- Avoids code duplication and ensures consistent behavior
- bootstrap focuses on boot-time setup; supervise handles runtime management
- The handoff is: mount rootfs → switch_root → exec supervise::execute() with config

**Alternatives considered:**

- Merge bootstrap into supervise crate: Would bloat supervise with boot-specific code
- Separate binary: Loses the single-binary simplicity that makes ocelot attractive

### 3. Unified YAML config: initrd options + supervise config

**Decision:** The initrd config file embeds supervise configuration as a nested section:

```yaml
# initrd-specific options
root:
  type: virtiofs
  tag: rootfs
  overlay: true

modules:
  - virtio_pci
  - virtio_blk

console: ttyS0

# embedded supervise config (same schema as supervise --file)
supervise:
  shutdown_timeout_secs: 30
  processes:
    - name: app
      command: /usr/bin/myapp
      ...
```

**Rationale:**

- Single config file for the entire boot-to-runtime lifecycle
- supervise config section reuses existing `SupervisorConfig` loading logic
- initrd options are minimal and QEMU-specific

**Alternatives considered:**

- Two separate config files: More complex, harder to manage
- CLI args for initrd, config file for supervise: Inconsistent UX

### 4. Use `nix` crate (not `rustix`) for syscalls

**Decision:** Continue using `nix` 0.31 (already in workspace) rather than adding `rustix`.

**Rationale:**

- `nix` already provides `mount()`, `finit_module()`, `chroot()`, `execv()`, `setsid()`
- No need to add another syscall wrapper dependency
- Consistent with existing codebase conventions
- `nix` 0.31 has all needed features enabled in workspace Cargo.toml

**Alternatives considered:**

- `rustix`: Used by kdf-init, more modern API, but adds dependency
- Direct `libc` calls: More error-prone, less type-safe

### 5. Kernel module loading via `finit_module()` syscall

**Decision:** Use `nix::sys::module::finit_module()` to load `.ko` files directly.

**Rationale:**

- No dependency on external `modprobe` binary (important for minimal initramfs)
- kdf-init demonstrates this pattern works well
- Can load modules from a known directory (e.g., `/lib/modules`)
- Graceful degradation: if module loading fails, log warning and continue

### 6. Error handling: panic with kmsg logging, optional debug shell

**Decision:** On fatal error, log to `/dev/kmsg` via tracing, then either spawn a debug shell (if configured) or loop indefinitely.

**Rationale:**

- initramfs environment has minimal tooling — can't rely on complex error recovery
- `/dev/kmsg` is always available and visible via `dmesg` or QEMU serial
- Debug shell option helps with VM development (matching kdf-init's approach)
- Loop is safer than reboot in production (preserves state for debugging)

### 7. No tokio in initrd boot phase

**Decision:** The initrd boot phase (mount, switch_root) uses synchronous code. After handoff, supervise uses tokio as normal.

**Rationale:**

- initrd boot is a linear sequence — no async benefit
- Reduces binary size (tokio is large)
- Simpler error handling in early boot
- supervise's tokio runtime is created inside `supervise::execute()` after handoff

## Risks / Trade-offs

| Risk                                                        | Mitigation                                                                                           |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Binary size too large for initramfs**                     | Use `musl` target with `opt-level = "z"` and `lto = true`; strip symbols; profile with `cargo-bloat` |
| **Device not ready when mounting**                          | Implement retry loop with timeout (similar to rsinit's `wait_for_device`)                            |
| **`finit_module()` fails for modules with dependencies**    | Load modules in dependency order; fall back to calling `modprobe` if available                       |
| **switch_root fails, leaving system in inconsistent state** | Log detailed error to kmsg; provide debug shell option for recovery                                  |
| **supervise config validation fails after switch_root**     | Validate config before switch_root; only hand off if config is valid                                 |
| **QEMU-specific features don't work on other hypervisors**  | Scope is QEMU-first; abstract storage backend trait for future extensibility                         |
