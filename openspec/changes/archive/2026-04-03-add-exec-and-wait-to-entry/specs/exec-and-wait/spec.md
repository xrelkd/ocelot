## ADDED Requirements

### Requirement: exec_and_wait function

The system SHALL provide an `exec_and_wait()` function that runs the supervisor event loop, waiting for a child process to exit and returning its exit code.

#### Scenario: Basic wait

- **WHEN** `exec_and_wait()` is called with a valid child PID, stdout_fd, stderr_fd, and timeout
- **THEN** the function blocks until the child process exits
- **AND** returns the child's exit code (or 128 + signal if terminated by signal)

#### Scenario: Signal forwarding during wait

- **WHILE** `exec_and_wait()` is waiting
- **WHEN** SIGINT or SIGTERM is received
- **THEN** the signal is forwarded to the child process
- **AND** a timeout begins for graceful shutdown

#### Scenario: Force kill after timeout

- **WHEN** child process does not exit within the configured timeout after receiving a signal
- **THEN** SIGKILL is sent to the child process

#### Scenario: I/O forwarding

- **WHILE** the child is running
- **WHEN** child produces output on stdout or stderr
- **THEN** the output is forwarded to the corresponding parent file descriptors

### Requirement: State management

The supervisor loop SHALL track process state including exit status, signal time, and kill status.

#### Scenario: Track exit

- **WHEN** child process exits
- **THEN** the exit status is captured and returned

#### Scenario: Track signal time

- **WHEN** a signal is forwarded to the child
- **THEN** the signal time is recorded for timeout enforcement
