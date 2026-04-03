## 1. Restructure BootstrapConfig (Drop ExecutionMode Enum)

- [ ] 1.1 Analyze current serde structure in `ocelot/src/config/bootstrap.rs`
- [ ] 1.2 Drop `ExecutionMode` enum, add `shell: Option<BootstrapShellConfig>` and `supervise: Option<BootstrapSuperviseConfig>` fields
- [ ] 1.3 Rename `ShellConfig` to `BootstrapShellConfig` in `ocelot/src/config/shell.rs`
- [ ] 1.4 Create `BootstrapSuperviseConfig` struct for supervise mode
- [ ] 1.5 Implement mutual exclusivity validation in `validate()` (exactly one of shell or supervise must be Some)
- [ ] 1.6 Update `to_bootstrap_config()`, `to_shell_config()`, `to_orchestrator_config()` methods for new structure
- [ ] 1.7 Test deserialization with shell mode YAML
- [ ] 1.8 Test deserialization with supervise mode YAML
- [ ] 1.9 Test deserialization with environment_variables and working_directory fields

## 2. Reorganize Template Files

- [ ] 2.1 Create `ocelot/src/config/templates/supervise/` directory
- [ ] 2.2 Create `templates/supervise/minimal.yaml` with Python HTTP server on port 55688
- [ ] 2.3 Rename `templates/basic.yaml` to `templates/supervise/basic.yaml`
- [ ] 2.4 Copy current basic.yaml to `templates/supervise/full.yaml`
- [ ] 2.5 Create `ocelot/src/config/templates/bootstrap/` directory
- [ ] 2.6 Create `templates/bootstrap/shell.yaml` template (shell mode with env vars and cwd)
- [ ] 2.7 Create `templates/bootstrap/supervise.yaml` template (supervise mode with example processes)
- [ ] 2.8 Update `SuperviseConfig::template_*()` paths in `ocelot/src/config/supervise.rs`
- [ ] 2.9 Add `BootstrapConfig::template_shell()` in `ocelot/src/config/bootstrap.rs`
- [ ] 2.10 Add `BootstrapConfig::template_supervise()` in `ocelot/src/config/bootstrap.rs`

## 3. Update Supervise config-template with --template flag

- [ ] 3.1 Add `TemplateTier` enum in `ocelot/src/cli/supervise.rs` (Minimal, Basic, Full)
- [ ] 3.2 Update `Commands::ConfigTemplate` to include `--template` argument
- [ ] 3.3 Update `config-template` handler to select template based on tier
- [ ] 3.4 Test `ocelot supervise config-template` outputs basic (default)
- [ ] 3.5 Test `ocelot supervise config-template --template minimal`
- [ ] 3.6 Test `ocelot supervise config-template --template basic`
- [ ] 3.7 Test `ocelot supervise config-template --template full`

## 4. Add Bootstrap CLI Subcommands (run, validate, config-template)

- [ ] 4.1 Update `Bootstrap` command in `ocelot/src/cli/mod.rs` to support subcommands
- [ ] 4.2 Create `Commands` enum in `ocelot/src/cli/bootstrap.rs` with variants: Run, Validate, ConfigTemplate
- [ ] 4.3 Implement `run` subcommand in bootstrap (same as current default behavior)
- [ ] 4.4 Verify `ocelot bootstrap` and `ocelot bootstrap run` both work
- [ ] 4.5 Add `validate` subcommand with `--file` and `--output` arguments
- [ ] 4.6 Add `config-template` subcommand with `--mode` argument
- [ ] 4.7 Update help text for bootstrap command

## 5. Add Bootstrap Validate Command Implementation

- [ ] 5.1 Implement `validate_config` function following supervise pattern
- [ ] 5.2 Support human and JSON output formats
- [ ] 5.3 Add unit tests for validate command
- [ ] 5.4 Test validation with valid config
- [ ] 5.5 Test validation with invalid YAML syntax
- [ ] 5.6 Test validation with missing required field
- [ ] 5.7 Test validation with both shell and supervise set (should fail)
- [ ] 5.8 Test validation with neither shell nor supervise set (should fail)
- [ ] 5.9 Test validation with duplicate environment variables

## 6. Add Bootstrap Config-Template Command Implementation

- [ ] 6.1 Implement shell mode template with environment_variables and working_directory
- [ ] 6.2 Implement supervise mode template with example processes
- [ ] 6.3 Add unit tests for config-template command
- [ ] 6.4 Test default template output (shell mode)
- [ ] 6.5 Test supervise mode template output
- [ ] 6.6 Verify templates include new configuration fields

## 7. Integration and Testing

- [ ] 7.1 Run cargo fmt to format code
- [ ] 7.2 Run cargo clippy to check for warnings
- [ ] 7.3 Run cargo test to ensure all tests pass
- [ ] 7.4 Test complete workflow: generate template → validate → verify works
- [ ] 7.5 Update any existing tests that reference old YAML format
- [ ] 7.6 Verify all new commands appear in `--help` output

## 8. Documentation

- [ ] 8.1 Update bootstrap CLI help text
- [ ] 8.2 Document new YAML configuration format
