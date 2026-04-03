## ADDED Requirements

### Requirement: Environment variables can be configured in bootstrap YAML

The system SHALL support an `environment_variables` field in the top-level bootstrap configuration. This field SHALL be an array of key-value pairs (strings). When multiple entries have the same key, the last occurrence SHALL win (override previous). If no `environment_variables` are specified, the system SHALL inherit the parent environment unchanged.

#### Scenario: Environment variables set from YAML

- **GIVEN** a bootstrap YAML configuration with `environment_variables: [["PATH", "/usr/bin"], ["LANG", "en_US.UTF-8"]]`
- **WHEN** ocelot bootstrap executes
- **THEN** the process environment SHALL have `PATH="/usr/bin"` and `LANG="en_US.UTF-8"`
- **AND** the environment variables SHALL be set after mounting filesystems but before any exec

#### Scenario: Duplicate environment variable keys

- **GIVEN** a bootstrap YAML configuration with `environment_variables: [["PATH", "/usr/bin"], ["PATH", "/bin"]]`
- **WHEN** configuration is loaded
- **THEN** the effective `PATH` SHALL be `/bin` (the last value)
- **AND** no error SHALL be raised during deserialization

#### Scenario: Environment variables inherit from parent

- **GIVEN** a bootstrap YAML configuration with `environment_variables: [["CUSTOM", "value"]]`
- **AND** the parent process environment has `PATH="/usr/local/bin:/usr/bin"`
- **WHEN** ocelot bootstrap executes
- **THEN** the environment SHALL contain both `CUSTOM="value"` and `PATH="/usr/local/bin:/usr/bin"`

### Requirement: Working directory can be configured in bootstrap YAML

The system SHALL support a `working_directory` field in the top-level bootstrap configuration. This field SHALL be an optional string representing an absolute path (relative to the new root filesystem). If `working_directory` is not specified, the current working directory SHALL remain unchanged (inherit from parent, typically `/`). If specified, the system SHALL change the current working directory to that path after mounting all filesystems and before switching root.

#### Scenario: Working directory is set to existing directory

- **GIVEN** a bootstrap YAML configuration with `working_directory: "/srv/app"`
- **AND** the directory `/srv/app` exists in the new root filesystem (mounted at `/newroot/srv/app`)
- **WHEN** ocelot bootstrap executes mounts root and overlay
- **THEN** `std::env::set_current_dir("/srv/app")` SHALL be called successfully
- **AND** after `switch_root`, the new process (shell or supervise) SHALL have its current working directory at `/srv/app`

#### Scenario: Working directory is not specified

- **GIVEN** a bootstrap YAML configuration without a `working_directory` field
- **WHEN** ocelot bootstrap executes
- **THEN** the current working directory SHALL be whatever was inherited from the parent process (typically `/`)
- **AND** no `chdir` call SHALL be made

#### Scenario: Working directory path does not exist

- **GIVEN** a bootstrap YAML configuration with `working_directory: "/nonexistent"`
- **AND** the path `/nonexistent` does not exist in the new root
- **WHEN** ocelot bootstrap attempts to change directory
- **THEN** the bootstrap SHALL fail with an error indicating inability to change directory
- **AND** the system SHALL not proceed to `switch_root`

### Requirement: Configuration validation prevents duplicate environment keys

The system SHALL validate `environment_variables` during configuration loading to ensure no duplicate keys exist in the array. If duplicates are found, the system SHALL return a configuration error with a message indicating the duplicate key.

#### Scenario: Duplicate environment variable keys detected

- **GIVEN** a bootstrap YAML configuration with `environment_variables: [["KEY", "value1"], ["KEY", "value2"]]`
- **WHEN** the configuration is deserialized and validated
- **THEN** the system SHALL return an error indicating that environment variable "KEY" appears multiple times
- **AND** the bootstrap process SHALL abort before attempting to mount filesystems

#### Scenario: No duplicates is valid

- **GIVEN** a bootstrap YAML configuration with `environment_variables: [["A", "1"], ["B", "2"]]`
- **WHEN** the configuration is loaded
- **THEN** no validation error SHALL occur

### Requirement: Environment and working directory apply to both shell and supervise modes

The `environment_variables` and `working_directory` configuration SHALL be applied identically in both `execute_shell` and `execute_supervise` modes. The settings SHALL be global to the process that is about to be executed (the shell or the supervise orchestrator), affecting its environment and initial working directory.

#### Scenario: Shell mode respects config

- **GIVEN** ocelot is configured with `mode: shell` and `environment_variables` and `working_directory` set
- **WHEN** `execute_shell` is invoked
- **THEN** before calling `switch_root_shell`, the environment variables SHALL be set and working directory SHALL be changed
- **AND** the shell process (exec'd) SHALL inherit these settings

#### Scenario: Supervise mode respects config

- **GIVEN** ocelot is configured with `mode: supervise` and `environment_variables` and `working_directory` set
- **WHEN** `execute_supervise` is invoked
- **THEN** before calling `switch_root_into`, the environment variables SHALL be set and working directory SHALL be changed
- **AND** the supervise orchestrator process (exec'd) SHALL inherit these settings

### Requirement: Backward compatibility

If a configuration does not include `environment_variables` or `working_directory`, the system SHALL behave exactly as before (no changes). The fields SHALL be optional with default values of empty list and `None` respectively.

#### Scenario: Configuration without new fields

- **GIVEN** a bootstrap YAML configuration from before this feature (no `environment_variables` or `working_directory`)
- **WHEN** ocelot bootstrap executes
- **THEN** the system SHALL function normally without setting any additional environment variables or changing directory
- **AND** no regressions SHALL occur
