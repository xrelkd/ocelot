## Why

When running ocelot bootstrap in initramfs, the config file path should be configurable via kernel command line parameters. Currently, the bootstrap subcommand only accepts `--file` or uses a hardcoded default path. Adding kernel command line support allows VM configurations to specify the bootstrap config path without modifying the command line arguments, which is useful for dynamic VM provisioning scenarios.

Additionally, `crates/bootstrap/src/cmdline.rs` contains unused `root_type` and `root_device` fields in `CmdlineParams` that are parsed but never consumed anywhere in the codebase. These should be removed to reduce maintenance burden.

## What Changes

- The ocelot CLI `bootstrap` subcommand will check for `ocelot.config=<path>` in the kernel command line when `--file` is not explicitly provided
- The `crates/bootstrap` crate will expose a function to parse and return the config file path from kernel command line
- **BREAKING**: The `--log-level` CLI option will be removed from the `bootstrap` subcommand
- **BREAKING**: Log level will be configured via `BootstrapConfig` YAML field instead of CLI
- Logging will be initialized in the ocelot CLI after parsing the config file, using the config's log level
- For `execute_shell` mode, log level will be fixed at `info` to show important information
- **BREAKING**: The `cmdline.rs` module will only parse `ocelot.config=` parameter; other parameters (`console`, `log_level`, `root_type`, `root_device`) will be removed as they are unused

## Capabilities

### New Capabilities

- `bootstrap-kernel-cmdline-config`: Support reading bootstrap config file path from kernel command line parameter `ocelot.config=<path>`

### Modified Capabilities

- `bootstrap-cli`: Update the bootstrap subcommand behavior to fall back to kernel command line config path before using the hardcoded default

## Impact

- `ocelot/src/cli/mod.rs`: Remove `--log-level` from `Bootstrap` command, modify to check kernel cmdline for config path, initialize logging after parsing config file
- `ocelot/src/config/bootstrap.rs`: Add `log_level` field to `BootstrapConfig`
- `crates/bootstrap/src/cmdline.rs`: Simplify to only parse `ocelot.config=` parameter, remove unused parameters (`console`, `log_level`, `root_type`, `root_device`)
- `crates/bootstrap/src/lib.rs`: Export config path parsing function, remove tracing-subscriber dependency
- `openspec/specs/bootstrap-cli/spec.md`: Update scenarios to cover kernel command line fallback and log level changes
