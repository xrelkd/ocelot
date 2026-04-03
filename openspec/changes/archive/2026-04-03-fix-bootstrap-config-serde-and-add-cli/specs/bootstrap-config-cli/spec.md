## ADDED Requirements

### Requirement: Bootstrap validate command validates configuration

The system SHALL provide an `ocelot bootstrap validate` command that validates a bootstrap YAML configuration file.

#### Scenario: Valid configuration file

- **GIVEN** a valid bootstrap YAML file with correct syntax and all required fields
- **WHEN** user runs `ocelot bootstrap validate --file config.yaml`
- **THEN** the command exits with code 0 and outputs "Configuration is valid"

#### Scenario: Invalid YAML syntax

- **GIVEN** a YAML file with syntax errors
- **WHEN** user runs `ocelot bootstrap validate --file invalid.yaml`
- **THEN** the command exits with code 1 and outputs an error message describing the YAML parse error

#### Scenario: Missing required field

- **GIVEN** a YAML file missing the required `root` field
- **WHEN** user runs `ocelot bootstrap validate --file missing-root.yaml`
- **THEN** the command exits with code 1 and outputs "missing field `root`"

#### Scenario: Duplicate environment variables

- **GIVEN** a bootstrap configuration with duplicate keys in `environmentVariables`
- **WHEN** user runs `ocelot bootstrap validate --file dup-env.yaml`
- **THEN** the command exits with code 1 and outputs "Bootstrap configuration has duplicate environment variables"

#### Scenario: JSON output format

- **GIVEN** a valid bootstrap YAML file
- **WHEN** user runs `ocelot bootstrap validate --file config.yaml --output json`
- **THEN** the command outputs `{"valid":true}` for valid configs or `{"valid":false,"error":"..."}` for invalid

### Requirement: Bootstrap config-template command outputs configuration template

The system SHALL provide an `ocelot bootstrap config-template` command that outputs a default bootstrap configuration template.

#### Scenario: Default template (shell mode)

- **WHEN** user runs `ocelot bootstrap config-template`
- **THEN** the command outputs a valid YAML configuration with shell mode, including the `environment_variables` and `working_directory` fields

#### Scenario: Supervise mode template

- **WHEN** user runs `ocelot bootstrap config-template --mode supervise`
- **THEN** the command outputs a valid YAML configuration with supervise mode and example processes

#### Scenario: Template includes new fields

- **WHEN** user runs `ocelot bootstrap config-template`
- **THEN** the output includes `environment_variables: []` (empty array) and `workingDirectory: null` (or omitted) fields to show users the full configuration options
