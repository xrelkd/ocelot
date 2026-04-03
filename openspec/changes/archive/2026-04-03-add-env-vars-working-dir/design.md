## Context

Ocelot's bootstrap process (in `crates/bootstrap/src/lib.rs`) currently:

1. Mounts virtual filesystems (proc, sys, dev, run)
2. Loads kernel modules (optional)
3. Mounts root filesystem (virtiofs/block/9p)
4. Optionally sets up overlayfs
5. Switches root and execs into target (shell or supervise orchestrator)

There is currently **no mechanism** to set environment variables or change the working directory before the exec. This is needed for:

- Setting PATH, LANG, and other environment variables required by applications
- Starting the supervise orchestrator or shell in a specific directory (e.g., `/srv`, `/home`)
- Providing container-like environment configuration via YAML

The supervise mode already has per-process environment_variables and working_directory in ProcessConfig, but there is **no global bootstrap-level** configuration that applies before the orchestrator starts (for supervise) or before the shell exec (for shell mode).

## Goals / Non-Goals

**Goals:**

- Add `environment_variables` and `working_directory` fields to `BootstrapConfig` in `crates/bootstrap/src/config.rs`
- Execute `std::env::set_var()` for each environment variable **after mounting all filesystems but before `switch_root()`**
- Execute `std::env::set_current_dir()` if `working_directory` is set **after mounting filesystems but before `switch_root()`**
- Make these settings apply globally to the process that will be exec'd (the shell or the supervise orchestrator)
- Support YAML deserialization with backward compatibility (fields optional)
- Validate environment variable keys for duplicates at config load time
- Add appropriate error handling (IoError for chdir)

**Non-Goals:**

- Modify per-process environment/working_directory in ProcessConfig (that remains separate)
- Support dynamic changes after switch_root (these are one-time bootstrap settings)
- Provide a way to unset/override environment variables from the parent environment (we only add/set)
- Support non-UTF8 environment variable names/values (we assume valid UTF-8)

## Decisions

### 1. Where to add the configuration fields?

**Decision**: Add to `BootstrapConfig` (in `crates/bootstrap/src/config.rs`), NOT to `ShellConfig` or `OrchestratorConfig`.

**Rationale**:

- The fields are meant to be global to the bootstrap process and apply regardless of execution mode (shell or supervise)
- The user's example shows they should be applied after mounting filesystems but before exec
- Placing them at `BootstrapConfig` level makes them available in both `execute_shell` and `execute_supervise` without duplication
- The YAML configuration (`ocelot/src/config/bootstrap.rs`) already contains a top-level `BootstrapConfig`; we'll extend it

### 2. When exactly to apply environment and cwd changes?

**Decision**: After all filesystem mounts (including overlay) are complete, but **before** calling `switch_root::switch_root_into()` or `switch_root::switch_root_shell()`.

**Rationale**:

- Must be after mounts because `working_directory` might refer to a path inside the new root filesystem (mounted at `/newroot`)
- Must be before switch_root because after `switch_root` the current working directory is changed automatically to `/` (by the kernel)
- Environment variables are process-wide and should be set before `execv()` so the new program inherits them
- This ordering matches the user's code snippet: mount → set env/cwd → switch_root → exec

**Implementation location**: In both `execute_supervise` and `execute_shell`, add a new step after `mount_overlay()` (or after mounts complete) and before the `switch_root_*` call.

### 3. How to handle environment variable validation?

**Decision**: Add a `validate()` method or run-time check in `BootstrapConfig::new()`/deserialization to detect duplicate keys, returning a `ConfigError::DuplicateEnvironmentVariable`.

**Rationale**:

- Duplicate environment variables are ambiguous; we should fail fast at config load time
- Similar to existing `ProcessConfig::validate_environment_variables()` pattern (see `ocelot/src/config/process.rs:210-228`)
- Provide clear error message with the duplicate key

### 4. How to handle `working_directory` path resolution?

**Decision**: Treat `working_directory` as a path that exists **inside the new root** (under `/newroot`). Implement `chdir()` after mounting root but before switch_root, using `std::env::set_current_dir(&config.working_directory)`.

**Rationale**:

- The user expects the working directory to be relative to the target root filesystem
- We can't resolve it to `/newroot/...` path because `chdir()` operates on the current mount namespace
- After mounting root at `/newroot`, the paths are directly accessible
- If the directory doesn't exist or isn't accessible, return an `IoError` with context

### 5. Error handling?

**Decision**: Create a new error variant in `crates/bootstrap/src/error.rs`: `FailedToChangeWorkingDirectory { path: String, source: std::io::Error }` with `#[snafu]` display message.

**Rationale**:

- Consistent with existing error handling patterns (Snafu)
- Provides actionable error message with path and underlying I/O error

### Alternatives Considered

- **Put fields in ShellConfig only**: Rejected because supervise mode also needs them globally
- **Apply env/cwd after switch_root**: Impossible because switch_root changes PID 1 and we lose control; also too late for exec inheritance
- **Use execvpe() with custom envp array instead of set_var**: Could work but would require changing all exec calls; set_var is simpler and more flexible (allows intermediate code to read env)
- **Resolve working_directory relative to old root**: Would be confusing; better to use new root paths directly

## Risks / Trade-offs

- **[Risk] Environment pollution**: If user sets `LD_PRELOAD` or `LD_LIBRARY_PATH`, it could affect the orchestrator or shell unexpectedly. → **Mitigation**: Document clearly; users are responsible for their env vars.
- **[Risk] working_directory path doesn't exist yet** (e.g., directory is created later by a process). → **Mitigation**: Check existence early? Not feasible because directory might be created by another process after boot. Better to let `chdir()` fail at boot time with clear error.
- **[Risk] Order dependency**: User might rely on env vars set by parent (e.g., from kernel command line). Our implementation **overwrites/extends** rather than clearing. This is fine because we only add/set.
- **[Risk] UTF-8 validation**: Environment variable names/values are `String`; invalid UTF-8 will fail deserialization. Acceptable as YAML should be valid UTF-8.
- **[Trade-off] No support for environment variable expansion** (e.g., `$PATH`). Simpler implementation; users must explicitly set all needed vars.

## Migration Plan

This is a purely additive change:

1. Introduce new optional fields in config structs with `#[serde(default)]`
2. Existing configurations without these fields continue to work unchanged (no env/cwd changes)
3. No data migration needed
4. Documentation update: describe new fields in configuration schema

**Rollback**: Remove the fields and the new code; configurations will simply ignore the fields.

## Open Questions

- Q: Should we allow `working_directory` to be an absolute path or also support relative paths?  
  **A**: Should be absolute path relative to new root; we won't enforce `.is_absolute()` but will rely on `chdir()` failure for invalid paths.
- Q: Should we allow empty environment variable values?  
  **A**: Yes, empty values are valid (e.g., `PATH=""` to clear).
- Q: Should we bail if `set_current_dir` fails, or just log a warning and continue?  
  **A**: Bail (return error) because starting in the wrong directory could cause cascading failures.
