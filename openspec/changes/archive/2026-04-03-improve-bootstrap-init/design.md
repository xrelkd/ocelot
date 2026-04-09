## Context

The `ocelot-bootstrap` crate is a minimal init system for QEMU VMs that handles early boot in initramfs. It currently mounts virtual filesystems, loads kernel modules, mounts the root filesystem, switches root, and hands off to the supervise orchestrator.

Two issues limit its effectiveness:

1. Console setup lacks `ioctl(TIOCSCTTY)`, so child processes don't have a controlling terminal
2. Overlay directories use a single `/run/overlay/` path, causing conflicts with multiple overlay mounts

A third enhancement adds a shell execution mode for debugging, which is mutually exclusive with supervise mode.

## Goals / Non-Goals

**Goals:**

- Add controlling terminal setup to console initialization
- Isolate overlay directories per mount source
- Add `execute_shell()` function for shell mode
- Make shell mode and supervise mode mutually exclusive in config
- CLI wrapper handles cmdline parsing and mode selection

**Non-Goals:**

- Kernel cmdline parsing in bootstrap crate (CLI handles this)
- Standalone bootstrap binary
- Dynamic module discovery/scanning
- Compressed module support (.ko.xz, .ko.gz)
- Changes to supervise orchestrator

## Decisions

### 1. Two mutually exclusive entry points

**Decision**: Provide `execute(config, orchestrator)` for supervise mode and `execute_shell(config, shell_config)` for shell mode. CLI decides which to call.

**Rationale**: Clear separation of concerns. The bootstrap crate doesn't need mode-switching logic; it receives a clear directive. Shell and supervise are fundamentally different paths.

**Config validation**: YAML validation ensures only one mode is configured. If both `shell` and `processes` are set, config parsing fails.

```rust
// In bootstrap crate
pub fn execute(config: &Config, orchestrator: OrchestratorConfig) -> Result<(), Error> { ... }
pub fn execute_shell(config: &Config, shell_config: &ShellConfig) -> Result<(), Error> { ... }
```

```yaml
# Valid: supervise mode
shell: null
processes:
  app: { command: ["/usr/bin/app"] }

# Valid: shell mode
shell:
  program: /bin/sh
  args: ["-i"]
processes: {}  # or omitted

# Invalid: both modes
shell: { program: "/bin/sh" }
processes:
  app: { command: ["/usr/bin/app"] }
```

### 2. ShellConfig struct

**Decision**: Create `ShellConfig` struct with `program` and `args` fields, separate from main `Config`.

**Rationale**: Clean separation. Shell config is only passed to `execute_shell()`, not mixed into the main config that flows through the boot sequence.

```rust
#[derive(Clone, Debug)]
pub struct ShellConfig {
    pub program: String,
    pub args: Vec<String>,
}
```

### 3. TIOCSCTTY via libc ioctl

**Decision**: Use libc ioctl with TIOCSCTTY constant after dup2.

**Rationale**: The nix crate's `dup2_raw` works but doesn't expose TIOCSCTTY. Using libc directly is straightforward for this single call.

### 4. Overlay directory structure

**Decision**: Change from `/run/overlay/{tag}/upper` to `/run/overlayfs/{source}/upper` where source is the mount identifier.

**Rationale**: When multiple mounts use overlay, they need isolated upper/work directories. The source identifier (virtiofs tag, block device name, 9p tag) uniquely identifies each mount.

**Implementation**:

```rust
fn overlay_base(source: &str) -> String {
    let safe_name = source.replace('/', "_");
    format!("/run/overlayfs/{safe_name}")
}
```

### 5. Keep nix, don't migrate to rustix

**Decision**: Continue using nix crate for syscalls.

**Rationale**: The codebase already uses nix extensively. Migrating to rustix would be a larger change with no immediate benefit. The ioctl call can be added via libc FFI if needed.

## Risks / Trade-offs

| Risk                                        | Mitigation                                               |
| ------------------------------------------- | -------------------------------------------------------- |
| TIOCSCTTY fails on non-terminal consoles    | Log warning, continue (boot shouldn't fail)              |
| Overlay path sanitization misses edge cases | Use conservative character whitelist for source names    |
| Shell execution leaves orphan processes     | Shell is the only process; system shutdown on shell exit |

## Migration Plan

No migration needed. The overlay path change is internal to initramfs and transparent to users. Existing configs continue to work.

For users who have scripts referencing `/run/overlay/` paths, those would need updating to `/run/overlayfs/{source}/`.
