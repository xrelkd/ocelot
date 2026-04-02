## Context

The ocelot bootstrap subcommand currently uses a hardcoded default config path (`/etc/ocelot/bootstrap.yaml`) when `--file` is not specified. In VM environments, it's useful to specify the config path via kernel command line parameters, allowing dynamic VM provisioning without modifying command line arguments.

The `crates/bootstrap/src/cmdline.rs` module already has infrastructure for parsing kernel command line parameters (`ocelot.*` prefixed options), but doesn't include config path parsing. Additionally, it contains unused `root_type` and `root_device` fields that add maintenance burden without providing value.

## Goals / Non-Goals

**Goals:**

- Add `ocelot.config=<path>` kernel command line parameter support
- Update bootstrap subcommand to use kernel cmdline config path as fallback
- Remove unused `root_type` and `root_device` from `CmdlineParams`
- Maintain backward compatibility with existing `--file` flag behavior
- Move log level configuration from CLI to YAML config file
- Fix log level to `info` for shell mode execution

**Non-Goals:**

- Changing config file format or structure (except adding log_level field)
- Modifying the supervise subcommand behavior

## Decisions

### Decision 1: Config path resolution order

The bootstrap subcommand will resolve config file path in this order:

1. `--file` flag (highest priority)
2. `ocelot.config=<path>` from kernel command line
3. Default `/etc/ocelot/bootstrap.yaml` (fallback)

**Rationale:** Explicit command line flags should always take precedence. Kernel cmdline provides a convenient middle ground for VM configurations where command line cannot be easily modified.

**Alternative considered:** Environment variable support. Rejected because kernel cmdline is more appropriate for initramfs context where environment may not be fully initialized.

### Decision 2: Public API location

Add `get_config_path()` function to `crates/bootstrap/src/cmdline.rs` that reads and parses the config path from `/proc/cmdline`. Only `ocelot.config=` parameter is parsed; other parameters are removed.

**Rationale:**

- The ocelot CLI imports and uses this function directly
- `crates/bootstrap` should not depend on `tracing-subscriber`; logging initialization belongs in the CLI layer
- Simplifies the cmdline module to single responsibility

**Alternative considered:** Adding config parsing in the CLI layer. Rejected because it would duplicate the cmdline reading logic.

### Decision 3: Simplify cmdline.rs

Remove all parameters except `ocelot.config=` from `CmdlineParams`. The removed parameters (`console`, `log_level`, `root_type`, `root_device`) are either unused or superseded by YAML config.

**Rationale:** These parameters are parsed but never consumed anywhere. The configuration values come from the YAML config file.

### Decision 4: Log level configuration

Move log level from CLI `--log-level` option to `BootstrapConfig` YAML field. Logging will be initialized in the ocelot CLI after parsing the config file, not inside `crates/bootstrap`.

**Rationale:**

- `crates/bootstrap` does not depend on `tracing-subscriber`
- The CLI has access to both the parsed config and tracing infrastructure
- Bootstrap runs as PID 1 in initramfs where CLI options are less flexible than config files

**Alternative considered:** Initialize logging inside `crates/bootstrap`. Rejected because it would add `tracing-subscriber` dependency to the library crate.

### Decision 5: Fixed log level for shell mode

Shell mode (`execute_shell`) will always use `info` log level, ignoring any configured value.

**Rationale:** Shell mode is for debugging purposes where showing important operational information is critical. Verbose debug output would clutter the interactive shell session.

## Risks / Trade-offs

- [Risk] Kernel cmdline might be unavailable in some test environments → Mitigation: The function returns `Option<String>`, gracefully handling missing/empty cmdline
- [Risk] Breaking change for users who have `ocelot.root.type` or `ocelot.root.device` in their kernel cmdline (currently unused) → Mitigation: These parameters were never functional; removal is safe

## Migration Plan

1. Simplify `crates/bootstrap/src/cmdline.rs` to only parse `ocelot.config=`
2. Add `log_level` field to `BootstrapConfig` in `ocelot/src/config/bootstrap.rs`
3. Remove `log_level` field from `Config` in `crates/bootstrap/src/config.rs` (logging done in CLI)
4. Remove `--log-level` from `Bootstrap` CLI subcommand
5. Export `get_config_path` function from `crates/bootstrap/src/lib.rs`
6. Update `ocelot/src/cli/mod.rs` to: check kernel cmdline, parse config, initialize logging, then call execute
7. Run tests to verify correctness

**Breaking change:** Users who currently use `--log-level` with `ocelot bootstrap` must migrate to adding `logLevel` field in their YAML config file.

## Open Questions

None
