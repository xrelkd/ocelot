## ADDED Requirements

### Requirement: System shutdown after bootstrap completion

The system SHALL perform a clean power-off after the bootstrap process completes.

#### Scenario: Shutdown after shell mode completion

- **WHEN** the shell exits in shell mode
- **THEN** the system SHALL call reboot(RB_AUTOBOOT) to power off

#### Scenario: Shutdown after supervise mode completion

- **WHEN** execute_supervise returns
- **THEN** the system SHALL call reboot(RB_AUTOBOOT) to power off

### Requirement: Shutdown error handling

The system SHALL return an error if the shutdown operation fails.

#### Scenario: Shutdown failure

- **WHEN** reboot() returns an error
- **THEN** the system SHALL return a Shutdown error with the underlying cause

### Requirement: CLI invokes shutdown

The CLI SHALL invoke the shutdown function after execute_shell or execute_supervise returns.

#### Scenario: CLI calls shutdown in shell mode

- **WHEN** execute_shell returns successfully
- **THEN** the CLI SHALL call shutdown() before exiting

#### Scenario: CLI calls shutdown in supervise mode

- **WHEN** execute_supervise returns successfully
- **THEN** the CLI SHALL call shutdown() before exiting
