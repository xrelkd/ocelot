## Why

Ocelot's bootstrap process currently mounts virtiofs shares and sets up the root filesystem, but provides no way to configure environment variables or the working directory before executing the shell or supervise orchestrator. This limits flexibility for users who need to set PATH, locale, or other environment variables, or who need the process to start in a specific directory after the root switch.

## What Changes

- Add `environment_variables` field to bootstrap configuration (Vec<(String, String)>)
- Add `working_directory` field to bootstrap configuration (Option<String>)
- Apply these settings via plain Rust types in `crates/bootstrap/src/config.rs` (NO serde - only ocelot config uses serde)
- Execute environment variable setup via `std::env::set_var()` after mounting filesystems but before switching root
- Execute working directory change via `std::env::set_current_dir()` after mounting filesystems but before switching root
- Apply these settings globally before executing either shell or supervise orchestrator
- Update YAML configuration schema in ocelot to include new optional fields with serde
- Add validation to prevent duplicate environment variable keys

## Capabilities

### New Capabilities

- `bootstrap-env-config`: Support environment variables and working directory configuration in the bootstrap phase

### Modified Capabilities

_(none)_

## Impact

- **Files to modify**:
  - `crates/bootstrap/src/config.rs` (BootstrapConfig struct - plain Rust types, NO serde)
  - `crates/bootstrap/src/lib.rs` (execute_shell, execute_supervise functions)
  - `ocelot/src/config/bootstrap.rs` (BootstrapConfig YAML deserialization with serde)
  - `crates/bootstrap/src/error.rs` (new error variant for chdir failures)
- **IMPORTANT**: Only configs in the `ocelot` crate are allowed to use serde. The `crates/bootstrap` crate must NOT have serde dependencies or serialize/deserialize implementations.
