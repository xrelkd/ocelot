## ADDED Requirements

### Requirement: Log rotation parameters validation

The system SHALL validate that all log rotation parameters, when present, are positive (greater than zero).

#### Scenario: Valid rotation parameters

- **WHEN** a process configuration includes log rotation with `maxSizeBytes: 10485760`, `rotationIntervalSecs: 86400`, `maxFiles: 7`, `maxAgeDays: 30`
- **THEN** the validation SHALL succeed for these fields

#### Scenario: Invalid maxSizeBytes

- **WHEN** `maxSizeBytes` is set to 0 or a negative value
- **THEN** the validation SHALL fail with an error indicating `maxSizeBytes` must be positive

#### Scenario: Invalid rotationIntervalSecs

- **WHEN** `rotationIntervalSecs` is set to 0 or a negative value
- **THEN** the validation SHALL fail with an error indicating `rotationIntervalSecs` must be positive

#### Scenario: Invalid maxFiles

- **WHEN** `maxFiles` is set to 0 or a negative value
- **THEN** the validation SHALL fail with an error indicating `maxFiles` must be positive

#### Scenario: Invalid maxAgeDays

- **WHEN** `maxAgeDays` is set to 0 or a negative value
- **THEN** the validation SHALL fail with an error indicating `maxAgeDays` must be positive

### Requirement: Enhanced cycle detection

The system SHALL improve the cyclic dependency error message to display the full dependency cycle path, not just the process name reported by the topological sort.

#### Scenario: Cycle with full path

- **WHEN** a configuration contains a dependency cycle such as A → B → C → A
- **THEN** the validation SHALL fail with an error that lists the processes in the cycle in order (e.g., "Cyclic dependency: A depends on B, B depends on C, C depends on A")
- **AND** the error SHALL help the user identify the exact loop to break

### Requirement: Rotation destination compatibility warning

The system SHALL emit a warning during validation when log rotation parameters are configured but the log destination type is `null` or `inherit` (where rotation has no effect).

#### Scenario: Rotation configured with null destination

- **WHEN** a process configuration has `log.stdout.destination.type = "null"` (or `"inherit"`) AND rotation parameters are set
- **THEN** the validation SHALL produce a warning message indicating that rotation is ineffective for the chosen destination type
- **AND** the validation SHALL still succeed (warning, not error)

#### Scenario: Rotation configured with inherit destination

- **WHEN** a process configuration has `log.stdout.destination.type = "inherit"` AND rotation parameters are set
- **THEN** the validation SHALL produce a warning message indicating that rotation is ineffective for inherited destinations
- **AND** the validation SHALL still succeed (warning, not error)

#### Scenario: Rotation configured with file destination

- **WHEN** a process configuration has `log.stdout.destination.type = "file"` with rotation parameters
- **THEN** the validation SHALL NOT produce any warning about destination type
- **AND** the rotation configuration SHALL be validated normally

#### Scenario: Both stdout and stderr checked independently

- **WHEN** both stdout and stderr have their own destination and rotation configurations
- **THEN** the validation SHALL check each stream independently and emit warnings as appropriate for each

### Requirement: Probe configuration validation

The system SHALL validate that probe configurations have sensible values: `timeout` ≤ `period`, and network ports are within the valid range 1-65535.

#### Scenario: Valid probe with timeout less than period

- **WHEN** a probe has `initialDelay: 5s`, `period: 30s`, `timeout: 5s`
- **THEN** the validation SHALL succeed

#### Scenario: Timeout greater than period

- **WHEN** a probe has `period: 30s` and `timeout: 35s`
- **THEN** the validation SHALL fail with an error indicating `timeout` must not exceed `period`

#### Scenario: Invalid HTTP probe port

- **WHEN** an HTTP probe specifies `port: 0` or `port: 70000`
- **THEN** the validation SHALL fail with an error indicating port must be in range 1-65535

#### Scenario: Invalid TCP probe port

- **WHEN** a TCP probe specifies `port: -1` or `port: 99999`
- **THEN** the validation SHALL fail with an error indicating port must be in range 1-65535

#### Scenario: Port not specified uses default

- **WHEN** a probe does not specify a port (uses default)
- **THEN** the validation SHALL pass (default port is assumed to be valid)

### Requirement: Restart policy validation

The system SHALL validate that restart policy backoff durations, if specified, are positive.

#### Scenario: Valid backoff

- **WHEN** a restart policy `Always` or `OnFailure` specifies `backoff: 5s`
- **THEN** the validation SHALL succeed

#### Scenario: Zero backoff

- **WHEN** a restart policy specifies `backoff: 0s`
- **THEN** the validation SHALL fail with an error indicating `backoff` must be positive

#### Scenario: Negative backoff

- **WHEN** a restart policy specifies `backoff: -2s`
- **THEN** the validation SHALL fail with an error indicating `backoff` must be positive

### Requirement: Process-level field validation

The system SHALL validate that essential process fields are present and have valid values.

#### Scenario: Empty program path

- **WHEN** a process configuration has `program: ""` or omits the program field
- **THEN** the validation SHALL fail with an error indicating `program` is required

#### Scenario: Invalid terminationGracePeriod

- **WHEN** `terminationGracePeriod` is set to 0 or a negative duration
- **THEN** the validation SHALL fail with an error indicating `terminationGracePeriod` must be positive

#### Scenario: Valid terminationGracePeriod

- **WHEN** `terminationGracePeriod` is set to a positive duration (e.g., `30s`)
- **THEN** the validation SHALL succeed

### Requirement: Validation error specificity

The system SHALL produce distinct error types for each validation failure to enable precise user feedback.

#### Scenario: Each validation failure produces a specific error

- **WHEN** any of the above validation checks fails
- **THEN** the error SHALL indicate the specific field and reason (e.g., "log rotation maxFiles must be positive, got 0")
- **AND** the error type SHALL uniquely identify the validation category (e.g., `InvalidLogRotation`, `InvalidProbe`)

### Requirement: Duplicate environment variable detection

The system SHALL detect and reject configurations that define the same environment variable name multiple times within a single process's `environmentVariables` map.

#### Scenario: Duplicate environment variable names

- **WHEN** a process configuration contains duplicate keys in `environmentVariables` (e.g., `FOO: bar` appears twice in YAML)
- **THEN** deserialization SHALL detect the duplication
- **AND** validation SHALL fail with an error listing the duplicate variable name(s)
- **AND** the configuration SHALL NOT be accepted

#### Scenario: Unique environment variables

- **WHEN** a process configuration has all unique environment variable names
- **THEN** the validation SHALL pass for the environmentVariables field

### Requirement: Rotation configuration consistency

The system SHALL validate that when log rotation is configured, at least one of `max_size_bytes` or `rotation_interval_secs` is set to a positive value (i.e., rotation must have at least one trigger). Both being zero or unset is invalid when rotation exists.

#### Scenario: Rotation with both size and interval zero

- **WHEN** a log rotation configuration has `maxSizeBytes: 0` and `rotationIntervalSecs: 0` (or both effectively zero)
- **THEN** the validation SHALL fail with an error indicating that at least one rotation trigger must be positive

#### Scenario: Rotation with size positive only

- **WHEN** rotation is configured with `maxSizeBytes > 0` and `rotationIntervalSecs: 0` (or omitted)
- **THEN** the validation SHALL succeed (size-based rotation)

#### Scenario: Rotation with interval positive only

- **WHEN** rotation is configured with `rotationIntervalSecs > 0` and `maxSizeBytes: 0` (or omitted)
- **THEN** the validation SHALL succeed (time-based rotation)

#### Scenario: Rotation with both positive

- **WHEN** rotation is configured with both `maxSizeBytes > 0` and `rotationIntervalSecs > 0`
- **THEN** the validation SHALL succeed (whichever triggers first)
