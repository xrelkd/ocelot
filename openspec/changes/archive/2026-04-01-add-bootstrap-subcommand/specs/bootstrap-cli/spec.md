## ADDED Requirements

### Requirement: Bootstrap subcommand

The CLI SHALL provide a `bootstrap` subcommand (with `boot` alias) that runs the bootstrap boot flow.

#### Scenario: Run bootstrap with config file

- **WHEN** `ocelot bootstrap --file /path/to/config.yaml` is executed
- **THEN** the bootstrap boot flow runs with the specified configuration

#### Scenario: Run bootstrap with boot alias

- **WHEN** `ocelot boot --file /path/to/config.yaml` is executed
- **THEN** the bootstrap boot flow runs with the specified configuration (same as `bootstrap`)

#### Scenario: Run bootstrap with default config

- **WHEN** `ocelot bootstrap` is executed without `--file`
- **THEN** the bootstrap boot flow runs with the default config path `/etc/ocelot/bootstrap.yaml`

#### Scenario: Bootstrap with custom log level

- **WHEN** `ocelot bootstrap --log-level debug` is executed
- **THEN** tracing output includes debug-level messages

#### Scenario: Bootstrap with environment log level

- **WHEN** `ocelot bootstrap` is executed with `OCELOT_LOG_LEVEL=warn` set
- **THEN** tracing output respects the environment variable

### Requirement: Bootstrap subcommand help

The CLI SHALL provide help text for the `bootstrap` subcommand describing its purpose and options.

#### Scenario: Show bootstrap help

- **WHEN** `ocelot bootstrap --help` is executed
- **THEN** help text describing the subcommand purpose, arguments, and options is displayed

### Requirement: Bootstrap config template

The CLI SHALL provide a `config-template` sub-subcommand under `bootstrap` that outputs a default configuration template.

#### Scenario: Output config template

- **WHEN** `ocelot bootstrap config-template` is executed
- **THEN** a YAML template with all bootstrap config fields and comments is written to stdout

### Requirement: Bootstrap config validation

The CLI SHALL provide a `validate` sub-subcommand under `bootstrap` that validates a configuration file without executing the boot flow.

#### Scenario: Valid config

- **WHEN** `ocelot bootstrap validate --file /path/to/config.yaml` is executed with a valid config
- **THEN** exit code 0 and "Configuration is valid" is printed

#### Scenario: Invalid config

- **WHEN** `ocelot bootstrap validate --file /path/to/invalid.yaml` is executed with an invalid config
- **THEN** exit code 1 and error details are printed to stderr

#### Scenario: JSON output

- **WHEN** `ocelot bootstrap validate --file /path/to/config.yaml --output json` is executed
- **THEN** validation result is output as JSON `{"valid": true}` or `{"valid": false, "errors": [...]}`
