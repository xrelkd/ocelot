## 1. Error Types

- [ ] 1.1 Add new `ValidationError` variants in `ocelot/src/config/error.rs`:
  - `InvalidLogRotation { field: String, value: i64 }`
  - `InvalidProbeTimeout { timeout: u64, period: u64 }`
  - `InvalidProbePort { port: u32 }`
  - `InvalidRestartBackoff { backoff: u64 }`
  - `InvalidTerminationGracePeriod { value: u64 }`
  - `MissingProcessProgram { process: String }`
  - `DuplicateEnvironmentVariables { process: String, variables: Vec<String> }`
  - `InvalidRotationConfiguration { reason: String }`
- [ ] 1.2 Ensure `Snafu` attributes generate proper display messages

## 2. Enhanced Validation Logic

- [ ] 2.1 Add `validate_process_config(&self, config: &ProcessConfig) -> Result<(), ValidationError>` to `SupervisorConfig` in `ocelot/src/config/mod.rs`
- [ ] 2.2 In `validate_process_config`, check `config.program` is non-empty; if empty return `MissingProcessProgram`
- [ ] 2.3 Check `config.termination_grace_period > 0`; if zero return `InvalidTerminationGracePeriod`
- [ ] 2.4 If `config.log` contains `rotation`, validate:
  - `max_size_bytes > 0`
  - `rotation_interval_secs > 0`
  - `max_files > 0`
  - `max_age_days > 0`
  - Additionally, ensure at least one of `max_size_bytes` or `rotation_interval_secs` is positive (both zero invalid)
  - Return appropriate `InvalidLogRotation` or `InvalidRotationConfiguration` on failure
- [ ] 2.5 If `config.readiness_probe` or `config.liveness_probe` is `Some`:
  - Check `timeout <= period`
  - If `handler` is HTTP or TCP, validate `port` in 1..=65535
  - Return `InvalidProbeTimeout` or `InvalidProbePort` as appropriate
- [ ] 2.6 If `config.restart_policy`:
  - For `Always` or `OnFailure` with backoff, validate `backoff > 0`
  - Return `InvalidRestartBackoff` if zero
- [ ] 2.7 Detect duplicate environment variable names in `config.environment_variables`:
  - Iterate over keys and check for duplicates
  - Return `DuplicateEnvironmentVariables` with list of duplicates if found
- [ ] 2.8 Validate rotation destination compatibility:
  - In `validate_process_config`, check if destination is `Null` or `Inherit` and rotation is configured
  - If so, emit a warning using `eprintln!` (not `tracing`) but do not fail
- [ ] 2.9 Call `validate_process_config` for each process from `SupervisorConfig::validate()` after existing checks (version, missing deps, cycle detection)
- [ ] 2.10 Replace existing `detect_dependency_cycles` implementation with enhanced version:
  - Keep using `toposort` to detect cycle
  - On failure, use `kosaraju_scc(&graph)` to get strongly connected components
  - Find the SCC containing the failing node
  - Perform DFS within that SCC to extract a cycle path
  - Return `Err(Error::Validate { source: ValidationError::CyclicDependency { cycle: Vec<String> } })` where cycle is list of process names in order
- [ ] 2.11 Update `CyclicDependency` variant in `ValidationError` to include `cycle: Vec<String>` field
- [ ] 2.12 Update display for `CyclicDependency` to format cycle as "A → B → C → A" (with arrows)
- [ ] 2.13 Add unit tests for each validation failure case

## 3. CLI Validate Subcommand

- [ ] 3.1 Add `Validate` variant to `Commands` enum in `ocelot/src/cli/supervise.rs` with args:
  - `file: PathBuf`
  - `output: Option<OutputFormat>` enum (Human default, Json)
- [ ] 3.2 Add handler in `supervise::run` for `Validate`:
  - Accept `file: PathBuf` argument
  - Load config via `SupervisorConfig::load(file)`
  - Call `config.validate()`
  - On success: use `println!("Configuration is valid")` to stdout and exit with 0
  - On failure: use `eprintln!` to stderr for errors (human by default, or JSON structure if `--output json`)
  - Exit with 1 on failure
  - IMPORTANT: Use `println!`/`eprintln!` directly, not `tracing` macros, for output control
- [ ] 3.3 Ensure CLI parsing accepts `validate` as subcommand with file path and optional `--output` flag
- [ ] 3.4 When `--output json`, emit JSON with fields: `{ "valid": boolean, "errors": [ { "message": string, "field": string? } ] }`

## 4. Serde Valid Integration

- [ ] 4.1 Add `serde_valid` dependency to `ocelot/Cargo.toml`
- [ ] 4.2 Annotate configuration fields with appropriate serde_valid attributes:
  - Port fields: `#[validate(range(min = 1, max = 65535))]`
  - Duration fields: `#[validate(range(min = 1))]`
  - Size fields: use custom validator that accepts both integer and string with bytesize parsing
- [ ] 4.3 Implement custom validator functions (e.g., `validate_size`) that parse human-readable sizes using bytesize and ensure positivity
- [ ] 4.4 Ensure deserialization errors from serde_valid are converted into `ValidationError` variants properly
- [ ] 4.5 Update error types to include context from serde_valid failures if needed
- [ ] 4.6 Write unit tests for serde_valid constraints on various fields

## 5. Human-Readable Sizes Support

- [ ] 5.1 Add `bytesize` dependency to `ocelot/Cargo.toml`
- [ ] 5.2 Modify config structs (or create newtypes) to support parsing size values that can be either `u64` or human-readable strings
- [ ] 5.3 Ensure that fields like `max_size_bytes` accept both integers and strings (e.g., using `serde_with` or custom deserializer)
- [ ] 5.4 The custom validator from serde_valid integration will handle parsing the string and converting to bytes
- [ ] 5.5 Add tests for valid human-readable sizes ("10MB", "1GB", "512KB") and invalid formats
- [ ] 5.6 Verify backward compatibility: plain integers continue to work

## 6. Tests

- [ ] 6.1 Add unit tests for enhanced validation:
  - Invalid log rotation values
  - Invalid probe timeout/port
  - Invalid restart backoff, termination grace period, missing program
  - Duplicate environment variables
  - Rotation both zero
  - Rotation destination warning
  - Enhanced cycle detection: verify cycle error includes full path (e.g., "A → B → C → A")
- [ ] 6.2 Add integration tests for `validate` subcommand:
  - Valid config exits 0 with success message on stdout
  - Invalid config exits 1 with error on stderr
  - `--output json` produces correct JSON structure
  - Rotation on null destination: exit 0 with warning on stderr
  - Dependency cycle: exit 1 with error containing cycle path
- [ ] 6.3 Test file-not-found and parse errors

## 7. Verification

- [ ] 7.1 Run `cargo test`
- [ ] 7.2 Run `cargo clippy`
- [ ] 7.3 Run `cargo fmt --check`
- [ ] 7.4 Manual testing with sample configs
