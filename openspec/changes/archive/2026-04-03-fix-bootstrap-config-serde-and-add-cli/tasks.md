## 1. Restructure BootstrapConfig (Drop ExecutionMode Enum)

- [x] 1.1 Analyze current serde structure in `ocelot/src/config/bootstrap.rs`
- [x] 1.2 Drop `ExecutionMode` enum, add `shell: Option<BootstrapShellConfig>` and `supervise: Option<BootstrapSuperviseConfig>` fields
- [x] 1.3 Create `BootstrapShellConfig` struct in `ocelot/src/config/shell.rs`
- [x] 1.4 Create `BootstrapSuperviseConfig` struct for supervise mode
- [x] 1.5 Implement mutual exclusivity validation in `validate()` (exactly one of shell or supervise must be Some)
- [x] 1.6 Update `to_bootstrap_config()`, `to_shell_config()`, `to_orchestrator_config()` methods for new structure
- [x] 1.7 Test deserialization with shell mode YAML
- [x] 1.8 Test deserialization with supervise mode YAML
- [x] 1.9 Test deserialization with environment_variables and working_directory fields

## 2. Reorganize Template Files

- [x] 2.1 Create `ocelot/src/config/templates/supervise/` directory
- [x] 2.2 Create `templates/supervise/minimal.yaml` with Python HTTP server on port 55688
- [x] 2.3 Rename `templates/basic.yaml` to `templates/supervise/basic.yaml`
- [x] 2.4 Copy current basic.yaml to `templates/supervise/full.yaml`
- [x] 2.5 Create `ocelot/src/config/templates/bootstrap/` directory
- [x] 2.6 Create `templates/bootstrap/shell.yaml` template (shell mode with env vars and cwd)
- [x] 2.7 Create `templates/bootstrap/supervise.yaml` template (supervise mode with example processes)
- [x] 2.8 Update `SuperviseConfig::template_*()` paths in `ocelot/src/config/supervise.rs`
- [x] 2.9 Add `BootstrapConfig::template_shell()` in `ocelot/src/config/bootstrap.rs`
- [x] 2.10 Add `BootstrapConfig::template_supervise()` in `ocelot/src/config/bootstrap.rs`

## 3. Update Supervise config-template with --template flag

- [x] 3.1 Add `TemplateTier` enum in `ocelot/src/cli/supervise.rs` (Minimal, Basic, Full)
- [x] 3.2 Update `Commands::ConfigTemplate` to include `--template` argument
- [x] 3.3 Update `config-template` handler to select template based on tier
- [x] 3.4 Test `ocelot supervise config-template` outputs basic (default)
- [x] 3.5 Test `ocelot supervise config-template --template minimal`
- [x] 3.6 Test `ocelot supervise config-template --template basic`
- [x] 3.7 Test `ocelot supervise config-template --template full`

## 4. Add Bootstrap CLI Subcommands (run, validate, config-template)

- [x] 4.1 Update `Bootstrap` command in `ocelot/src/cli/mod.rs` to support subcommands
- [x] 4.2 Create `Commands` enum in `ocelot/src/cli/bootstrap.rs` with variants: Run, Validate, ConfigTemplate
- [x] 4.3 Implement `run` subcommand in bootstrap (same as current default behavior)
- [x] 4.4 Verify `ocelot bootstrap` and `ocelot bootstrap run` both work
- [x] 4.5 Add `validate` subcommand with `--file` and `--output` arguments
- [x] 4.6 Add `config-template` subcommand with `--mode` argument
- [x] 4.7 Update help text for bootstrap command

## 5. Add Bootstrap Validate Command Implementation

- [x] 5.1 Implement `validate_config` function following supervise pattern
- [x] 5.2 Support human and JSON output formats
- [x] 5.3 Add unit tests for validate command
- [x] 5.4 Test validation with valid config
- [x] 5.5 Test validation with invalid YAML syntax
- [x] 5.6 Test validation with missing required field
- [x] 5.7 Test validation with both shell and supervise set (should fail)
- [x] 5.8 Test validation with neither shell nor supervise set (should fail)
- [x] 5.9 Test validation with duplicate environment variables

## 6. Add Bootstrap Config-Template Command Implementation

- [x] 6.1 Implement shell mode template with environment_variables and working_directory
- [x] 6.2 Implement supervise mode template with example processes
- [x] 6.3 Add unit tests for config-template command
- [x] 6.4 Test default template output (shell mode)
- [x] 6.5 Test supervise mode template output
- [x] 6.6 Verify templates include new configuration fields

## 7. Integration and Testing

- [x] 7.1 Run cargo fmt to format code
- [x] 7.2 Run cargo clippy to check for warnings
- [x] 7.3 Run cargo test to ensure all tests pass
- [x] 7.4 Test complete workflow: generate template → validate → verify works
- [x] 7.5 Update any existing tests that reference old YAML format
- [x] 7.6 Verify all new commands appear in `--help` output

## 8. Documentation

- [x] 8.1 Update bootstrap CLI help text
- [x] 8.2 Document new YAML configuration format
