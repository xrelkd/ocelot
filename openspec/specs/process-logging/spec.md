## ADDED Requirements

### Requirement: Process log destination configuration

The system SHALL allow users to configure the log destination for each process's stdout and stderr streams independently. The configuration SHALL be part of the process definition in the configuration file.

#### Scenario: Configure stdout to file

- **WHEN** a user specifies a `log.stdout.destination.file` path in the process configuration
- **THEN** the process's stdout SHALL be written to that file
- **AND** if rotation is configured, the file SHALL be rotated according to the rotation policy

#### Scenario: Configure stderr to null

- **WHEN** a user sets `log.stderr.destination.type = "null"` in the process configuration
- **THEN** the process's stderr output SHALL be discarded

#### Scenario: Configure both streams to inherit

- **WHEN** a user sets both stdout and stderr destinations to `inherit`
- **THEN** the process's stdout and stderr SHALL be inherited from the supervisor's stdout and stderr

#### Scenario: Missing log configuration defaults to inherit

- **WHEN** no `log` configuration is provided for a process
- **THEN** both stdout and stderr SHALL default to `inherit` destination
- **AND** the process's output SHALL appear on the supervisor's stdout/stderr as before

### Requirement: Log destination types

The system SHALL support three destination types for each log stream:

- `null`: Discard all output
- `inherit`: Inherit the supervisor's stdout/stderr
- `file(path)`: Write to specified file path

#### Scenario: Destination type null mutes output

- **WHEN** a stream is configured with destination type `null`
- **THEN** all data written to that stream by the process SHALL be discarded
- **AND** no file SHALL be created

#### Scenario: Destination type inherit uses supervisor's stream

- **WHEN** a stream is configured with destination type `inherit`
- **THEN** the child process's file descriptor SHALL effectively reference the same output stream as the supervisor's corresponding file descriptor (via piping)

#### Scenario: Destination type file writes to path

- **WHEN** a stream is configured with destination type `file` and a given path
- **THEN** the system SHALL create the file if it does not exist
- **AND** all output SHALL be written to that file
- **AND** if the file is rotated, subsequent output SHALL go to the new file

#### Scenario: Combining stdout and stderr into a single file

- **WHEN** both stdout and stderr are configured with destination `file` pointing to the same path
- **THEN** the system SHALL write both streams to that file
- **AND** the output from each stream MAY be interleaved
- **AND** each stream's rotation configuration SHALL be applied independently, which MAY cause races if configurations differ
- **AND** therefore the user SHOULD ensure both rotation configurations are identical when using a shared file

### Requirement: File rotation configuration

When the destination is `file`, the system SHALL support optional rotation policies to manage log file size and age. Rotation configuration SHALL include:

- `maxSizeBytes`: Rotate when file exceeds this size (optional)
- `rotationIntervalSecs`: Rotate based on time interval in seconds (e.g., 3600 for hourly, 86400 for daily) (optional)
- `maxFiles`: Maximum number of rotated files to retain; older files SHALL be deleted (optional, unlimited if omitted)

Rotation SHALL occur when either size or time condition is met (whichever comes first).

#### Scenario: Size-based rotation

- **WHEN** `maxSizeBytes` is set and the current log file size exceeds that threshold
- **THEN** the system SHALL rotate the file immediately before writing more data
- **AND** the current file SHALL be closed and renamed with a timestamp suffix
- **AND** a new log file SHALL be opened with the original path
- **AND** if `maxFiles` is set and the number of rotated files exceeds it, the oldest rotated file SHALL be deleted

#### Scenario: Time-based rotation

- **WHEN** `rotationIntervalSecs` is set and the current time exceeds the rotation interval since last rotation
- **THEN** the system SHALL rotate the file immediately
- **AND** the current file SHALL be closed and renamed with a timestamp suffix representing the rotation period
- **AND** a new log file SHALL be opened with the original path
- **AND** if `maxFiles` is set and exceeds limit, oldest file SHALL be deleted

#### Scenario: No rotation configuration

- **WHEN** destination is `file` but no rotation parameters are provided
- **THEN** the system SHALL write to the file indefinitely without rotation
- **AND** the file SHALL grow without bound (user manages rotation externally)

#### Scenario: Rotation with both size and time

- **WHEN** both `maxSizeBytes` and `rotationIntervalSecs` are configured
- **THEN** rotation SHALL occur when either condition is met first
- **AND** after rotation, both timers and size counters SHALL be reset

### Requirement: Rotated file naming

Rotated files SHALL be named using the pattern: `{original_path}.{timestamp}` where timestamp format depends on the rotation interval:

- Daily: `YYYY-MM-DD` (e.g., `app.log.2024-01-15`)
- Hourly: `YYYY-MM-DD-HH` (e.g., `app.log.2024-01-15-14`)
- Size-based: use the rotation timestamp in the same format as the time interval (if configured) or use the current timestamp at rotation moment if no time interval

#### Scenario: Daily rotation file naming

- **WHEN** a file is rotated with a daily time interval
- **THEN** the rotated file name SHALL be `{original_path}.YYYY-MM-DD`
- **AND** the format SHALL follow ISO 8601 date

#### Scenario: Size-based rotation without time interval

- **WHEN** a file is rotated due to size and no time interval is configured
- **THEN** the rotated file name SHALL include the current timestamp at rotation moment (e.g., `app.log.1673781234` using Unix timestamp or `app.log.2024-01-15T10-33-54`)
- **AND** the naming SHALL guarantee uniqueness to avoid overwriting

### Requirement: Configuration schema

The configuration YAML/JSON schema SHALL extend `ProcessConfig` with an optional `log` field:

```yaml
log:
  stdout:
    destination:
      type: null|inherit|file
      path: /path/to/file # only if type=file
    rotation:
      maxSizeBytes: 10485760 # optional, e.g., 10MB
      rotationInterval: 86400 # optional, e.g., 3600 (1h), 86400 (1d), human-readable like "1h", "24h"
      maxFiles: 5 # optional
  stderr:
    # same structure as stdout
```

#### Scenario: Full example configuration

- **WHEN** a user provides the following configuration:

```yaml
log:
  stdout:
    destination:
      type: file
      path: /var/log/myapp/stdout.log
    rotation:
      maxSizeBytes: 10485760
      rotationInterval: 24h
      maxFiles: 7
      maxAgeDays: 30
      mode: "640"
  stderr:
    destination:
      type: file
      path: /var/log/myapp/stderr.log
    rotation:
      maxSizeBytes: 10485760
```

- **THEN** stdout SHALL be written to `/var/log/myapp/stdout.log` with daily or size-based rotation (whichever first), keeping 7 files
- **AND** rotated files older than 30 days SHALL be auto-deleted
- **AND** stdout log file SHALL have permissions 640
- **AND** stderr SHALL be written to `/var/log/myapp/stderr.log` with size-based rotation only, keeping unlimited files

### Requirement: Backward compatibility

The `log` configuration field SHALL be optional. Existing configuration files without the `log` field SHALL continue to work unchanged, with default behavior of both streams inheriting the supervisor's stdout and stderr.

#### Scenario: Legacy configuration without log field

- **WHEN** a user's configuration file does not contain a `log` section
- **THEN** the process SHALL run with stdout and stderr both set to `inherit` destination
- **AND** the supervisor's behavior SHALL be as before
