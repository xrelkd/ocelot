## ADDED Requirements

### Requirement: Shell execution entry point

The system SHALL provide an `execute_shell()` function that performs the bootstrap boot flow and spawns an interactive shell instead of the supervise orchestrator.

#### Scenario: Execute shell mode

- **WHEN** `execute_shell()` is called with config and shell_config
- **THEN** the system mounts virtual filesystems, loads kernel modules, mounts rootfs, switches root, and spawns the specified shell

#### Scenario: Shell process setup

- **WHEN** spawning the shell process
- **THEN** the console device is opened with O_RDWR | O_CLOEXEC, setsid is called, and TIOCSCTTY establishes the controlling terminal

#### Scenario: Shell with arguments

- **WHEN** shell_config contains `program` and `args`
- **THEN** the shell is spawned with the specified arguments (e.g., `/bin/sh -i`)

#### Scenario: Shell exit

- **WHEN** the shell process exits
- **THEN** `execute_shell()` returns to the caller (shutdown is handled by CLI)

### Requirement: Shell and supervise mode exclusivity

The system SHALL enforce that shell mode and supervise mode are mutually exclusive in the configuration.

#### Scenario: Shell mode only

- **WHEN** config specifies `shell` but no `processes`
- **THEN** shell mode is active

#### Scenario: Supervise mode only

- **WHEN** config specifies `processes` but no `shell`
- **THEN** supervise mode is active (calls `execute()`)

#### Scenario: Both modes specified

- **WHEN** config specifies both `shell` and non-empty `processes`
- **THEN** configuration validation fails with an error indicating mutual exclusivity
