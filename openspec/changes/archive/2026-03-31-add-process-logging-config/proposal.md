## Why

Users need fine-grained control over how process stdout and stderr are handled. Currently, ocelot always splices process output to its own stdout/stderr. This limits use cases where users want to mute output, redirect to files with rotation, or explicitly inherit parent streams. Adding configurable logging provides flexibility for different deployment scenarios and log management strategies.

## What Changes

- Add a `log` configuration section to `ProcessConfig` with per-stream settings
- Support log destinations: `null` (mute), `inherit` (supervisor's stdout/stderr), `file` (write to path)
- Add file rotation configuration (size-based, time-based, or both)
- Modify `executor.rs` to respect log configuration when spawning processes
- Update command building to handle different output destinations

## Capabilities

### New Capabilities

- `process-logging`: Configurable stdout/stderr handling with file rotation support

### Modified Capabilities

- (None - this is a pure addition)

## Impact

- **Config**: `ocelot/src/config/process.rs` - add LogConfig struct
- **Supervisor**: `crates/supervise/src/supervisor/executor.rs` - modify spawn logic
- **Command**: Likely `crates/supervise/src/supervisor/command.rs` or similar for output handling
- **API**: New structs and enums in config module
- **CLI**: No changes to CLI interface
- **Breaking**: None - new optional field with defaults preserves existing behavior
