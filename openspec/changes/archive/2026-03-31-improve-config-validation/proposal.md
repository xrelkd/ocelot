## Why

The current configuration validation is limited to version checking, missing dependency detection, and cycle detection. There is no dedicated CLI command to validate configuration files without starting the supervisor. Additional validation for log rotation parameters, probe configurations, restart policies, and other fields is needed to catch misconfigurations early and provide clear feedback to users.

## What Changes

- Add a new `validate` subcommand to `ocelot supervise` that loads and validates a configuration file, returning exit code 0 for valid configs and non-zero for invalid ones.
- Enhance `SupervisorConfig::validate()` with additional checks:
  - Log rotation parameters: positive values for `max_size_bytes`, `rotation_interval_secs`, `max_files`, `max_age_days`
  - Probe configurations: `timeout` ≤ `period`, valid HTTP/TCP probe parameters
  - Restart policies: valid backoff durations
  - Process-level: ensure required fields are present, program path format validity
- Introduce new error types in the error hierarchy for detailed validation feedback.

## Capabilities

### New Capabilities

- `cli-validate-config`: Standalone configuration validation via `ocelot supervise validate <config-file>`
- `enhanced-config-validation`: Comprehensive validation of all configuration fields beyond dependency checks
- `human-readable-sizes`: Support human-readable size strings (e.g., "10MB", "1GB") using the bytesize crate
- `serde-valid`: Use serde_valid crate for declarative field-level validation

### Modified Capabilities

_(none)_

## Impact

- Modified files: `ocelot/src/config/mod.rs` (enhanced `validate` method), `ocelot/src/cli/supervise.rs` (add Validate subcommand), `ocelot/src/error.rs` (new error variants)
- No breaking changes; fully backward compatible
- Users gain a convenient way to validate configs before deployment and receive more actionable error messages
