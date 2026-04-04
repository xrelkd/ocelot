## Why

The `ocelot-bootstrap` crate is missing several capabilities needed in practice for QEMU VM boot scenarios. Specifically: multiple virtiofs mounts with per-share overlayfs, directory-scanning kernel module loading, virtiofs support pre-flight checks, symlink creation, and optional boot script execution. Users who need these features currently lack a structured, config-driven solution. This change extends bootstrap to cover these use cases so it becomes the single init solution.

## What Changes

- **Multiple virtiofs mounts**: Add `extra_virtiofs_mounts` field to config, mount each share with optional per-share overlayfs
- **Auto-scan kernel modules**: Convert `ModulesConfig` to an enum with `List` and `Scan` variants, supporting auto-discover `.ko` files in scan mode
- **Virtiofs support detection**: Check `/proc/filesystems` before attempting virtiofs mount, fail early with clear error
- **Symlink creation**: Add `symlinks` field to config, create symlinks during boot after root switch
- **Boot script execution**: Add `boot_script` field to config, optionally run a script before supervisor handoff or shell spawn
- **Module loading by directory scan**: Support loading all `.ko`/`.ko.xz`/`.ko.gz` files from a directory

## Capabilities

### New Capabilities

- `extra-virtiofs-mounts`: Mount multiple virtiofs shares in addition to the root filesystem, each with optional per-share overlayfs for writable layers
- `module-scan-mode`: Auto-discover and load all kernel module files from a directory; `ModulesConfig` becomes an enum with `List` and `Scan` variants
- `virtiofs-support-detection`: Pre-flight check that validates kernel virtiofs support before attempting mount operations
- `symlink-creation`: Create filesystem symlinks during the bootstrap boot flow from configuration specifications
- `boot-script-execution`: Optionally execute a boot script before handing off to the supervise orchestrator or spawning a shell

### Modified Capabilities

- `bootstrap-boot`: Extend boot flow requirements to include new stages (extra mounts, symlinks, script execution) and virtiofs support detection
- `bootstrap-config`: Extend config schema with new fields (`extra_virtiofs_mounts`, `symlinks`, `boot_script`, `ModulesConfig` converted to enum with `List`/`Scan` variants)

## Impact

- **`crates/bootstrap/src/config.rs`**: New structs (`VirtiofsMount`, `SymlinkSpec`, `BootScriptConfig`), `ModulesConfig` converted to enum (`List`/`Scan`), new fields on `Config`. Library types do NOT derive serde — they are pure data structures.
- **`ocelot/src/config/bootstrap.rs`**: CLI-side config structs with `Deserialize` derives for new fields. Separate from library types; conversion via `From`/`TryFrom`.
- **`crates/bootstrap/src/mount.rs`**: New functions for extra virtiofs mounts and per-share overlayfs
- **`crates/bootstrap/src/modules.rs`**: Scan-mode module loading alongside existing list-mode, dispatched from `ModulesConfig` enum
- **`crates/bootstrap/src/lib.rs`**: Extended `execute_supervise` and `execute_shell` with new boot stages
- **New file `crates/bootstrap/src/symlinks.rs`**: Symlink creation logic
- **New file `crates/bootstrap/src/script.rs`**: Boot script execution logic (uses `entry::execute` for zombie reaping, signal forwarding, and timeout support)
- **`crates/bootstrap/Cargo.toml`**: No new dependencies (all capabilities use existing `nix` crate)
- **Existing specs**: `bootstrap-boot` and `bootstrap-config` need delta requirements
- **New specs**: One spec file per new capability under `specs/`
