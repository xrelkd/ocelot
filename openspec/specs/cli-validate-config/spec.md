## ADDED Requirements

### Requirement: Validate configuration via CLI

The system SHALL provide a CLI subcommand `ocelot supervise validate <config-file>` that validates the configuration file without starting the supervisor. The command SHALL support configurable output formats.

#### Scenario: Valid configuration

- **WHEN** a user runs `ocelot supervise validate /path/to/valid-config.yaml`
- **THEN** the command SHALL load and parse the configuration file
- **AND** run all validation checks (including enhanced validation)
- **AND** output a success message to **stdout** indicating the configuration is valid
- **AND** exit with status code 0

#### Scenario: Invalid configuration

- **WHEN** a user runs `ocelot supervise validate /path/to/invalid-config.yaml`
- **THEN** the command SHALL load and parse the configuration file
- **AND** run all validation checks
- **AND** output one or more error messages to **stderr** describing the validation failures
- **AND** exit with a non-zero status code (1)

#### Scenario: Missing configuration file

- **WHEN** a user runs `ocelot supervise validate /nonexistent/file.yaml`
- **THEN** the command SHALL report to **stderr** that the file cannot be opened
- **AND** exit with a non-zero status code

#### Scenario: Invalid YAML/JSON syntax

- **WHEN** a user provides a configuration file with syntax errors
- **THEN** the command SHALL report the parsing error to **stderr** with line/column information
- **AND** exit with a non-zero status code

#### Scenario: JSON output format

- **WHEN** a user runs `ocelot supervise validate --output json /path/to/config.yaml`
- **THEN** the command SHALL produce machine-readable JSON output on stdout
- **AND** the JSON SHALL include fields such as `valid: true/false`, `errors: []` (if any), and any relevant details
- **AND** exit with status code 0 for valid configs, 1 for invalid

#### Scenario: Default output is human-readable

- **WHEN** a user runs `ocelot supervise validate /path/to/config.yaml` without `--output`
- **THEN** the output SHALL be human-readable plain text (as described in previous scenarios)
