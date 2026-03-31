## ADDED Requirements

### Requirement: Declarative field-level validation with serde_valid

The system SHALL use the `serde_valid` crate to add declarative validation constraints directly on configuration struct fields, reducing manual validation code and ensuring deserialization-time checks.

#### Scenario: Port number range validation

- **WHEN** a probe configuration defines a port field with attribute `#[validate(range(min = 1, max = 65535))]`
- **THEN** deserialization SHALL automatically validate that the port is within the valid range
- **AND** if the value is out of range, a validation error SHALL be produced with clear field context

#### Scenario: Positive duration validation

- **WHEN** fields like `terminationGracePeriod`, `rotationIntervalSecs`, `probe.period`, or `probe.timeout` are annotated with `#[validate(range(min = 1))]`
- **THEN** deserialization SHALL ensure these durations are greater than zero
- **AND** deserialization SHALL fail early with a specific error if a non-positive value is provided

#### Scenario: Custom validator for size strings

- **WHEN** a field like `maxSizeBytes` accepts either an integer or a human-readable string, and is annotated with `#[validate(custom = "validate_size")]`
- **THEN** deserialization SHALL invoke the custom validation function `validate_size`
- **AND** the custom function SHALL parse human-readable formats using the bytesize crate
- **AND** the custom function SHALL return an error for invalid formats or non-positive results

#### Scenario: Validation preserves existing error handling

- **WHEN** deserialization fails due to a `serde_valid` constraint
- **THEN** the error SHALL be converted into the existing `Error::Validate` with a `ValidationError` variant
- **AND** the CLI `validate` subcommand SHALL display the error message to stderr

#### Scenario: Integration with manual checks

- **WHEN** certain validations require cross-field context (e.g., `timeout <= period`), these SHALL remain in the manual `validate()` method
- **THEN** the `serde_valid` attributes handle simple per-field constraints
- **AND** the `SupervisorConfig::validate()` method continues to perform cross-field and dependency validation

#### Scenario: Backward compatibility

- **WHEN** a configuration is valid under the current schema
- **THEN** adding `serde_valid` constraints SHALL not break that configuration
- **AND** only previously undetected invalid values (e.g., negative ports, zero durations) SHALL cause new validation failures
