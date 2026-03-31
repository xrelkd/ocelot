## 1. Config Module Changes (ocelot/src/config/)

- [x] 1.1 Add `humantime` dependency to `ocelot/Cargo.toml` with `serde` feature
- [x] 1.2 Update `ProcessConfig` in `process.rs`:
  - Change `termination_grace_period_secs: u64` → `termination_grace_period: Duration`
  - Add `#[serde(with = "humantime::serde")]` attribute
  - Update default function to return `Duration::from_secs(60)`
- [x] 1.3 Update `ProbeConfig` in `probe.rs`:
  - Change `initial_delay_secs: u64` → `initial_delay: Duration`
  - Change `period_secs: u64` → `period: Duration`
  - Change `timeout_secs: u64` → `timeout: Duration`
  - Add `#[serde(with = "humantime::serde")]` to each
  - Update default constants to use `Duration` (e.g., `Duration::from_secs(10)`)
- [x] 1.4 Update `RestartPolicyConfig` in `restart.rs`:
  - In `Always` and `OnFailure` variants, rename field `backoff_secs` → `backoff` and type `Option<u64>` → `Option<Duration>`
  - Add `#[serde(with = "humantime::serde")]`
  - Update tests to use human-readable strings or Duration
- [x] 1.5 Add new logging config types:

  ```rust
  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
  pub enum LogDestination {
      Null,
      Inherit,
      File { path: PathBuf },
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(rename_all = "camelCase", deny_unknown_fields)]
  pub struct LogRotationConfig {
      pub max_size_bytes: Option<u64>,
      #[serde(with = "humantime::serde")]
      pub rotation_interval: Option<Duration>,
      pub max_files: Option<u32>,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(rename_all = "camelCase", deny_unknown_fields)]
  pub struct LogStreamConfig {
      pub destination: LogDestination,
      #[serde(default)]
      pub rotation: Option<LogRotationConfig>,
  }

  #[derive(Clone, Debug, Deserialize, Serialize)]
  #[serde(rename_all = "camelCase", deny_unknown_fields)]
  pub struct LogConfig {
      pub stdout: LogStreamConfig,
      pub stderr: LogStreamConfig,
  }
  ```

- [x] 1.6 Add optional `log: Option<LogConfig>` field to `ProcessConfig` with `#[serde(default)]`
- [x] 1.7 Update all config tests to use human-readable duration strings (e.g., "30s", "1h", "24h") instead of integers
- [x] 1.8 Add tests for deserialization of time fields with humantime (Duration)

## 2. Supervisor Config Integration (crates/supervise/src/supervisor/)

- [x] 2.1 Create new module `supervisor/log_config` (file: `crates/supervise/src/supervisor/log_config.rs`) with runtime (non-serde) types:
  - `LogDestination` enum: `Null`, `Inherit`, `File(PathBuf)`
  - `LogRotationConfig` struct: `max_size_bytes: Option<u64>`, `rotation_interval_secs: Option<u64>`, `max_files: Option<u32>`
  - `LogStreamConfig` struct: `destination: LogDestination`, `rotation: Option<LogRotationConfig>`
- [x] 2.2 Add fields `log_stdout: LogStreamConfig` and `log_stderr: LogStreamConfig` to `supervisor::Config` (in `config.rs`), with default values (both `LogStreamConfig { destination: LogDestination::Inherit, rotation: None }`)
- [x] 2.3 In the **application layer** (e.g., `ocelot/src/main.rs` or config loader), when building `supervisor::Config` from a `ProcessConfig`:
  - [x] 2.3.1 If `ProcessConfig::log` is `Some(log_cfg)`, map `log_cfg.stdout` to `supervisor::Config::log_stdout` and `log_cfg.stderr` to `log_stderr`
  - [x] 2.3.2 Use `supervisor::log_config::LogDestination::File` for file paths, `Inherit` for inherit, `Null` for null
  - [x] 2.3.3 If `ProcessConfig::log` is `None`, keep supervisor defaults (`Inherit`)
- [x] 2.4 Modify `supervisor::Config::command()` to set `discard_stdout`/`discard_stderr`:
  - [x] 2.4.1 If `self.log_stdout.destination == LogDestination::Null`, call `.discard_stdout(true)`
  - [x] 2.4.2 Otherwise `discard_stdout = false` (pipe)
  - [x] 2.4.3 Same for stderr

## 3. File Logging Task (TaskRunner)

- [x] 3.1 Add `register_file_logging` method to `TaskRunner` trait (in `task_runner.rs`) with signature:
  ```rust
  fn register_file_logging(
      &mut self,
      cancel_token: CancellationToken,
      event_sender: &mpsc::UnboundedSender<Event>,
      source_fd: OwnedFd,
      file_path: impl AsRef<Path> + Send,
      rotation: Option<LogRotationConfig>,
  );
  ```
- [x] 3.2 Implement for `JoinSet<()>`:
  - [x] 3.2.1 Spawn task that opens file, creates `RotatingFile`, and copies from `source_fd`
  - [x] 3.2.2 Use `AsyncFd` on `source_fd` to read data
  - [x] 3.2.3 Write chunks to `RotatingFile`, handling rotation under the hood
  - [x] 3.2.4 After opening the file and before starting the copy loop, send `Event::LogReady` (mirroring splice relay)
  - [x] 3.2.5 Clean up resources on EOF or cancellation (no extra event needed)

## 4. RotatingFile Implementation (crates/supervise/src/rotating_file/)

- [x] 4.1 Create `crates/supervise/src/rotating_file/mod.rs`
- [x] 4.2 Define `RotatingFile` struct with fields:
  - `base_path: PathBuf`
  - `current_file: tokio::fs::File`
  - `rotation: supervisor::log_config::LogRotationConfig` (or copy fields)
  - `current_size: u64`
  - `last_rotation: std::time::SystemTime`
- [x] 4.3 Implement `tokio::io::AsyncWrite` for `RotatingFile`:
  - [x] 4.3.1 Before write, check if rotation needed:
    - Size condition: `current_size + buf.len() > rotation.max_size_bytes` (if `max_size_bytes` is Some)
    - Time condition: If `rotation.rotation_interval_secs` is Some(interval), let `elapsed_secs = SystemTime::now().duration_since(last_rotation)?.as_secs()`; rotate if `elapsed_secs > interval`
    - Rotate if either condition is true
  - [x] 4.3.2 If rotation needed, call `rotate()` (which closes, renames, opens new file, resets counters)
  - [x] 4.3.3 Write buffer to `current_file` via `tokio::io::AsyncWriteExt::write_all`, update `current_size`
- [x] 4.4 Implement `rotate()`:
  - [x] 4.4.1 Drop current file
  - [x] 4.4.2 Generate timestamp suffix: if `rotation_interval_secs` is Some(secs), determine format (daily if secs >= 86400, hourly if secs >= 3600, else unix seconds); if None, use unix seconds
  - [x] 4.4.3 Rename `base_path` to `format!("{}.{}", base_path.display(), timestamp)` using `tokio::fs::rename`
  - [x] 4.4.4 If `max_files` set, list files matching `base_path.with_extension("")`? Actually pattern: `base_path.*`. Use `tokio::fs::read_dir`, parse timestamps, sort, delete oldest beyond limit
  - [x] 4.4.5 Open new `current_file` at `base_path` with ` tokio::fs::OpenOptions::new().append(true).create(true).open().await?`
  - [x] 4.4.6 Reset `current_size = 0`, `last_rotation = SystemTime::now()`
- [x] 4.5 In `new(base_path, rotation)`: open file, get metadata to initialize `current_size` and `last_rotation` (from `modified` time? Need to decide: for size-based only, use current size; for time-based, use mtime as last_rotation? Possibly use SystemTime::now() as last_rotation on first open, but if file already exists we might want to treat it as already rotated? Simpler: always treat initial open as fresh start; set `last_rotation = SystemTime::now()` and `current_size = 0` if file is new, or if existing, set `current_size = metadata.len()` and `last_rotation = SystemTime::now()` (so time interval from now). Alternatively, use modification time to approximate when rotation should happen. We'll define: On startup, we don't know when last rotation occurred; we can use file's modification time as `last_rotation` to honor time-based rotation based on file age. Then `current_size = 0`? Actually if file already exists, it may contain old data; we'll continue appending; `current_size` should be metadata.len() so we know when to rotate based on total size. And `last_rotation` could be metadata.modified()? That would mean time interval counts from last modification, which could trigger rotation prematurely? Better: we want rotation based on how long the current "active" file has been receiving writes. If we append to an existing file, we don't know when it was last rotated; we could assume it was rotated at its modification time and treat that as start of new period. That's reasonable. So: `last_rotation = metadata.modified()?`. Implement accordingly.
- [x] 4.6 Add unit tests (use tempfile, simulate writes, advance time with `tokio::time::pause` or mock time)

## 5. Executor Integration (executor.rs)

- [x] 5.1 In `ProcessSpawnContext::spawn()`, after obtaining `stdout_fd`/`stderr_fd`, check destination from `config`
- [x] 5.2 For `stdout`:
  - [x] 5.2.1 If `config.log_stdout.destination == Inherit` => call `tasks.register_splice_relay(..., Destination::Stdout)`
  - [x] 5.2.2 If `config.log_stdout.destination == File` => call `tasks.register_file_logging(..., file_path, rotation_config)`
  - [x] 5.2.3 If `Null` => nothing (fd already None)
- [x] 5.3 Same for `stderr` with `Destination::Stderr` and stderr config
- [x] 5.4 Ensure that if `File` destination but `stdout_fd` is None (unexpected), log error and skip

## 6. Testing

- [x] 6.1 Unit tests for `RotatingFile` (already in 4.6)
- [x] 6.2 Integration test: process with stdout->File, write enough data to trigger size rotation, verify files created with correct naming
- [x] 6.3 Integration test: time-based rotation using short interval (e.g., 1 second) and verify multiple files created
- [x] 6.4 Integration test: `max_files` limit enforced (rotate until > max, verify oldest deleted)
- [x] 6.5 Integration test: `Null` destination produces no file and output discarded
- [x] 6.6 Integration test: `Inherit` destination appears on supervisor's stdout/stderr
- [x] 6.7 Ensure all tests pass with `cargo nextest-all`

## 7. Documentation and Cleanup

- [x] 7.1 Add rustdoc to new public config types and `RotatingFile`
- [x] 7.2 Update config examples (if any) to show new `log` section
- [x] 7.3 Run `cargo fmt --all --check` and `cargo clippy-all`; fix any warnings
- [x] 7.4 Run `cargo doc-all` to ensure documentation builds
- [x] 7.5 Verify backward compatibility: existing configs without `log` work unchanged

## 8. Optional Enhancements

- [x] 8.1 Add compression for rotated logs (gzip)
- [x] 8.2 Add `max_age_days` for auto-deletion
- [ ] 8.3 Add log line prefixing (timestamp, PID) via optional `prefix` field
- [x] 8.4 Support for file creation mode (permissions) in config
