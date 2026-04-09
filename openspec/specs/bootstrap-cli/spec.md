## ADDED Requirements

### Requirement: Bootstrap subcommand

The CLI SHALL provide a `bootstrap` subcommand (with `boot` alias) that runs the bootstrap boot flow.

#### Scenario: Run bootstrap with config file

- **WHEN** `ocelot bootstrap --file /path/to/config.yaml` is executed
- **THEN** the bootstrap boot flow runs with the specified configuration

#### Scenario: Run bootstrap with boot alias

- **WHEN** `ocelot boot --file /path/to/config.yaml` is executed
- **THEN** the bootstrap boot flow runs with the specified configuration (same as `bootstrap`)

#### Scenario: Run bootstrap with kernel cmdline config

- **WHEN** `ocelot bootstrap` is executed without `--file` and kernel cmdline contains `ocelot.config=/custom/path.yaml`
- **THEN** the bootstrap boot flow runs with `/custom/path.yaml`

#### Scenario: Run bootstrap with default config fallback

- **WHEN** `ocelot bootstrap` is executed without `--file` and kernel cmdline does not contain `ocelot.config`
- **THEN** the bootstrap boot flow runs with the default config path `/etc/ocelot/bootstrap.yaml`

### Requirement: Bootstrap log level configuration

The CLI SHALL NOT provide a `--log-level` option for the bootstrap subcommand. Log level SHALL be configured via the `BootstrapConfig` YAML file.

#### Scenario: Log level from config file

- **WHEN** `ocelot bootstrap --file config.yaml` is executed and config contains `logLevel: debug`
- **THEN** tracing output includes debug-level messages during supervise mode

#### Scenario: Default log level from config

- **WHEN** `ocelot bootstrap --file config.yaml` is executed and config does not contain `logLevel`
- **THEN** tracing output uses default level `info`

#### Scenario: Shell mode uses info log level

- **WHEN** `ocelot bootstrap --file config.yaml` is executed in shell mode
- **THEN** tracing output is fixed at `info` level regardless of config setting

### Requirement: Remove --log-level option from bootstrap

The CLI SHALL NOT accept the `--log-level` option on the `bootstrap` subcommand.

#### Scenario: --log-level option is rejected

- **WHEN** `ocelot bootstrap --log-level debug` is executed
- **THEN** the command fails with an error indicating unknown option

### Requirement: Remove unused kernel cmdline parameters

The system SHALL remove the unused `root_type` and `root_device` parameters from kernel command line parsing.

#### Scenario: Unused parameters are not parsed

- **WHEN** `/proc/cmdline` contains `ocelot.root.type=block ocelot.root.device=/dev/vda2`
- **THEN** these parameters are ignored (not parsed into `CmdlineParams`)

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
