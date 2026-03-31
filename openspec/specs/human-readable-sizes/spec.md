## ADDED Requirements

### Requirement: Human-readable size configuration

The system SHALL allow users to specify size values (e.g., log rotation `maxSizeBytes`) using human-readable string representations such as "10MB", "1GB", "512KB" in addition to raw byte integers.

#### Scenario: Valid human-readable size with kilobytes

- **WHEN** a user specifies `maxSizeBytes: "512KB"` in log rotation configuration
- **THEN** the system SHALL parse the value using the bytesize crate
- **AND** interpret it as 512 \* 1024 = 524,288 bytes
- **AND** validation SHALL treat it as equivalent to `maxSizeBytes: 524288`

#### Scenario: Valid human-readable size with megabytes

- **WHEN** a user specifies `maxSizeBytes: "10MB"` in log rotation configuration
- **THEN** the system SHALL parse the value using the bytesize crate
- **AND** interpret it as 10 \* 1,048,576 = 10,485,760 bytes
- **AND** validation SHALL treat it as equivalent to `maxSizeBytes: 10485760`

#### Scenario: Valid human-readable size with gigabytes

- **WHEN** a user specifies `maxSizeBytes: "2GB"` in log rotation configuration
- **THEN** the system SHALL parse the value using the bytesize crate
- **AND** interpret it as 2 \* 1,073,741,824 = 2,147,483,648 bytes
- **AND** validation SHALL treat it as equivalent to `maxSizeBytes: 2147483648`

#### Scenario: Valid plain integer (backward compatibility)

- **WHEN** a user specifies `maxSizeBytes: 10485760` as a plain integer (no quotes)
- **THEN** the system SHALL accept it exactly as before
- **AND** validation SHALL treat it as 10,485,760 bytes

#### Scenario: Invalid size unit

- **WHEN** a user specifies `maxSizeBytes: "10XB"` with an unrecognized unit
- **THEN** the parsing SHALL fail
- **AND** validation SHALL return an error indicating an invalid size format

#### Scenario: Invalid size syntax

- **WHEN** a user specifies `maxSizeBytes: "tenMB"` with non-numeric prefix
- **THEN** the parsing SHALL fail
- **AND** validation SHALL return an error indicating the value cannot be parsed

#### Scenario: Case-insensitive units

- **WHEN** a user specifies `maxSizeBytes: "10mb"` (lowercase) or `"10Mb"` or `"10MB"`
- **THEN** the system SHALL parse all variants correctly using bytesize's case-insensitive parsing
- **AND** treat them as equivalent (10MB)

#### Scenario: Size zero and negative values

- **WHEN** a user specifies `maxSizeBytes: "0B"` or `maxSizeBytes: "-1MB"`
- **THEN** the validation SHALL reject these as invalid (must be positive)
- **AND** return an appropriate error indicating the size must be positive
