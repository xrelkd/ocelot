## 1. Update crates/bootstrap/src/cmdline.rs

- [x] 1.1 Simplify cmdline.rs to only parse `ocelot.config=` parameter
- [x] 1.2 Remove `console`, `log_level`, `root_type`, `root_device` parameters
- [x] 1.3 Add public `get_config_path()` function
- [x] 1.4 Update tests for new behavior

## 2. Update crates/bootstrap/src/lib.rs

- [x] 2.1 Export `get_config_path` function from the public API
- [x] 2.2 Remove tracing-subscriber dependency (logging done in CLI)

## 3. Update crates/bootstrap/src/config.rs

- [x] 3.1 Remove `log_level` field from `Config` struct (logging done in CLI)

## 4. Update ocelot/src/config/bootstrap.rs

- [x] 4.1 Add `log_level: tracing::Level` field to `BootstrapConfig` with default value `info`
- [x] 4.2 Remove log_level from `to_bootstrap_config()` conversion

## 5. Update ocelot/src/cli/bootstrap.rs

- [x] 5.1 Initialize logging after parsing config file
- [x] 5.2 Use config's log_level for supervise mode
- [x] 5.3 Use fixed `info` log level for shell mode

## 6. Update ocelot/src/cli/mod.rs

- [x] 6.1 Remove `--log-level` option from `Bootstrap` subcommand
- [x] 6.2 Modify `Bootstrap` command handling to check kernel cmdline when `--file` is not provided
- [x] 6.3 Remove `init_tracing_subscriber` call before bootstrap (logging handled in bootstrap.rs)

## 7. Verify and Test

- [x] 7.1 Run `cargo nextest run` to verify all tests pass
- [x] 7.2 Run `cargo clippy-all` to verify no lint errors
- [x] 7.3 Run `cargo fmt --all --check` to verify formatting
