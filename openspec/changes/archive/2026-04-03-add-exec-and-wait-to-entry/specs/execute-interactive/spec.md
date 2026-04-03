## ADDED Requirements

### Requirement: execute_interactive function

The system SHALL provide an `execute_interactive()` function that spawns an interactive shell with proper terminal setup, waits for it to exit, and returns the exit code.

#### Scenario: Spawn shell with terminal

- **WHEN** `execute_interactive()` is called with console path, shell program, args, and timeout
- **THEN** a new process is created via fork
- **AND** terminal setup is performed (setsid, dup2, TIOCSCTTY)
- **AND** the shell is executed via exec

#### Scenario: Terminal setup

- **WHEN** setting up the interactive shell
- **THEN** the console device is opened with O_RDWR | O_CLOEXEC
- **AND** setsid() is called to create a new session
- **AND** console is dup2'd to stdin, stdout, stderr
- **AND** TIOCSCTTY ioctl is called to establish controlling terminal

#### Scenario: Wait for shell exit

- **WHILE** the shell is running
- **WHEN** the shell process exits
- **THEN** the exit status is captured
- **AND** any remaining zombies are reaped
- **AND** the exit code is returned to the caller

#### Scenario: Signal handling

- **WHILE** waiting for the shell
- **WHEN** SIGINT or SIGTERM is received
- **THEN** the signal is forwarded to the shell process

#### Scenario: Timeout enforcement

- **WHEN** a signal is sent to the shell and it does not exit within the timeout
- **THEN** SIGKILL is sent to force termination
- **AND** the exit code (128 + SIGKILL) is returned

### Requirement: Console device handling

The function SHALL handle console device paths flexibly.

#### Scenario: Absolute console path

- **WHEN** console path starts with '/'
- **THEN** the path is used as-is

#### Scenario: Relative console path

- **WHEN** console path does not start with '/'
- **THEN** "/dev/" is prepended to the path
