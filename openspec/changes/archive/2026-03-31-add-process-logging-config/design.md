## Context

Ocelot currently always splices process stdout/stderr to its own stdout/stderr (see `executor.rs` lines 223-240). The configuration (`ProcessConfig`) lacks any log handling options. Users need to control where process output goes: mute it, inherit supervisor's stdout/stderr, or write to files with rotation.

The existing architecture uses:

- Custom `Command` type (in `crates/supervise/src/command.rs`) with `discard_stdout`/`discard_stderr` flags
- `SpliceRelay` to transfer data from process file descriptors to destinations (stdout, stderr, or raw fd)
- `tasks.register_splice_relay()` for setting up these transfers

## Goals / Non-Goals

**Goals:**

- Provide per-stream (stdout, stderr) configuration
- Support three destinations: `null` (discard), `inherit` (supervisor's stdout/stderr), `file` (write to path)
- Add file rotation: size-based, time-based, or both
- Preserve existing behavior when log config is not specified (default to `inherit`)
- Use existing `Command` flags where possible; avoid modifying `Command` struct
- Keep `SpliceRelay` unchanged; implement file logging as separate supervisor task
- Build `RotatingFile` using nix crate for file operations

**Non-Goals:**

- Remote log aggregation
- Log formatting or filtering
- Compression of rotated logs
- Windows support

## Decisions

### 1. LogConfig Structure (in `ocelot/src/config/process.rs`)

Add to `ProcessConfig` an optional field:

```rust
#[serde(default)]
pub log: Option<LogConfig>
```

where `LogConfig` contains two fields:

```rust
pub stdout: LogStreamConfig
pub stderr: LogStreamConfig
```

### 2. Log Stream Configuration

Define `LogStreamConfig` (in ocelot config):

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogStreamConfig {
    pub destination: LogDestination,
    #[serde(default)]
    pub rotation: Option<LogRotationConfig>,
}
```

`LogRotationConfig`:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogRotationConfig {
    pub max_size_bytes: Option<u64>,
    pub rotation_interval: Option<Duration>, // human-readable: "1h", "24h", "7d"
    pub max_files: Option<u32>,
}
```

Use `#[serde(with = "humantime::serde")]` for `Duration` fields to allow human-readable strings.

**Note:** This change also updates existing time fields across config to use `Duration` with humantime:

- `ProcessConfig::termination_grace_period_secs` → `termination_grace_period: Duration`
- `ProbeConfig::initial_delay_secs` → `initial_delay: Duration`, `period_secs` → `period: Duration`, `timeout_secs` → `timeout: Duration`
- `RestartPolicyConfig::{Always, OnFailure}::backoff_secs` → `backoff: Duration`

This is a breaking change for existing configs; migration requires converting integer seconds to human strings (e.g., `30` → `30s`, `3600` → `1h`).

Rotation only applies when `destination` is `File`. For `Null` and `Inherit`, `rotation` is ignored.

### 3. LogDestination Enum (named variants, no tuples, serde)

In `ocelot/src/config/process.rs` (or new module `log`):

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum LogDestination {
    Null,                    // Discard output
    Inherit,                 // Use supervisor's stdout/stderr
    File { path: PathBuf },  // Write to file
}
```

The `rename_all = "lowercase"` ensures YAML uses `null`, `inherit`, `file`.

### 4. Command Configuration (no modification to Command struct)

In `supervisor::Config::command()` (existing method in `config.rs`), we set discard flags based on log destination:

- `Null` => `.discard_stdout(true)` (or `discard_stderr`)
- `Inherit` => `.discard_stdout(false)` (default, pipes created for relay)
- `File` => `.discard_stdout(false)` (pipes created to capture output)

Thus, `Null` uses existing `Command` discard functionality; `Inherit` and `File` retain default (pipe) behavior.

### 5. Supervisor Config Extension

Add to `supervisor::Config`:

```rust
pub log_stdout: LogStreamConfig,
pub log_stderr: LogStreamConfig,
```

These fields are config-only (no serde). The supervisor converts from `ProcessConfig`'s `LogConfig` (if present) to these fields during config loading in the application layer (not in `supervisor::Config`). If `ProcessConfig::log` is `None`, both default to `Inherit`.

### 6. Executor Adjustments

In `executor.rs::ProcessSpawnContext::spawn()` (lines 208-256), after spawning and obtaining `stdout_fd` and `stderr_fd`:

- For `stdout_fd`:
  - If `config.log_stdout.destination == LogDestination::Inherit` => call `tasks.register_splice_relay(..., Destination::Stdout)` (existing behavior)
  - If `config.log_stdout.destination == LogDestination::File` => call `tasks.register_file_logging(..., file_path.as_ref(), rotation_config)` where file_path is from the `File` variant
  - If `Null` => do nothing (fd already None due to discard)
- Same for `stderr_fd` with `Destination::Stderr` and stderr config.

### 7. File Logging Task

Add a new method to `TaskRunner` trait (in `task_runner.rs`) and its impl for `JoinSet`:

```rust
fn register_file_logging(
    &mut self,
    cancel_token: CancellationToken,
    event_sender: &mpsc::UnboundedSender<Event>,
    source_fd: OwnedFd,
    file_path: impl AsRef<Path>,
    rotation: Option<LogRotationConfig>,
);
```

This spawns a task that:

- Opens (or creates) the target file using tokio::fs::File (append mode)
- Creates a `RotatingFile` wrapper (in `supervisor/rotating_file.rs`) managing async writes and rotation
- Reads from `source_fd` (using `AsyncFd`) and writes to `RotatingFile`
- Sends `Event::LogReady` **once the file is opened and ready** (mirroring `register_splice_relay` semantics)
- On EOF or cancellation, closes resources (no special event sent; LogReady already indicated readiness)

### 8. RotatingFile Implementation

Create new module `supervisor/rotating_file.rs`:

```rust
pub struct RotatingFile {
    base_path: PathBuf,
    current_file: tokio::fs::File,
    rotation: LogRotationConfig,
    // tracking current size, last rotation time, etc.
}
```

Implement `tokio::io::AsyncWrite` for `RotatingFile`:

- `write()` first checks if rotation is needed (`current_size + buf.len() > max_size_bytes` OR `SystemTime::now().duration_since(last_rotation).map(|d| d.as_secs()) > rotation.rotation_interval.map(|d| d.as_secs())` where last_rotation is timestamp of last rotation)
- If rotation needed, performs rotation:
  - Close current file (drop)
  - Generate timestamped filename: `{base_path}.{YYYY-MM-DD}` for daily, `{base_path}.{YYYY-MM-DD-HH}` for hourly, or for size-only use `{base_path}.{timestamp}` (unix seconds or formatted)
  - Rename current log file to timestamped name using `tokio::fs::rename` (atomic)
  - If `max_files` set, delete oldest rotated files (by sorting timestamps and keeping N newest)
  - Open new file at `base_path` in append mode (use `tokio::fs::OpenOptions::new().append(true).create(true)`)
- Then write data to new file
- Track bytes written since last rotation

### 9. Backward Compatibility

- Existing configs without `log` default to `Inherit` for both streams -> same pipe+splice behavior.
- No changes to CLI or external APIs.
- No changes to SpliceRelay.

## Risks / Trade-offs

- **Performance**: File I/O buffered; rotation adds occasional pause. Acceptable for logging.
- **Concurrency**: Multiple processes may write to same log file. No coordination; user's responsibility.
- **Complexity**: New file relay task and rotation logic increase codebase. But isolated to supervisor.
- **Disk Space**: Unlimited rotation by default; user must manage via `max_files` or external tools.

## Migration Plan

1. Add log config types to `ocelot/src/config/process.rs`
2. Extend `supervisor::Config` with log fields and conversion from `ProcessConfig`
3. Update `Config::command()` to set discard for `Null`
4. Add `register_file_logging` to `TaskRunner` trait and `JoinSet` impl
5. Implement `RotatingFile` in new module
6. Modify `executor.rs::ProcessSpawnContext` to branch on destination
7. Add unit tests for `RotatingFile` and integration tests for logging configs
8. Run full test suite; ensure no regressions

Rollback: Remove log field usage; defaults preserve old behavior. No config migration needed.

## Open Questions

- Should `register_file_logging` be a new trait method? Yes, to keep abstraction.
- Should `RotatingFile` compress rotated logs? Non-goal now.
- Timestamp format: Use `%Y-%m-%d` for daily, `%Y-%m-%d-%H` for hourly; for size-only, use unix timestamp or fallback to time-based naming with current time.
- Should rotation happen pre-write or post-write? Pre-write ensures file never exceeds max size; easier to track current size and rotate when about to exceed.
- What time zone for timestamp? UTC to avoid DST issues.
- Should we support `max_age_days`? Not in initial version.
- Should we handle errors during rotation gracefully (e.g., disk full)? Log error and continue writing to current file if possible; maybe fallback to no rotation.
