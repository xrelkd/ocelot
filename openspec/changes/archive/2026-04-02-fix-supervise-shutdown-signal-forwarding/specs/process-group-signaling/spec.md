## ADDED Requirements

### Requirement: Supervised processes run in their own process group

Each supervised process SHALL be placed in its own process group via `setpgid(0, 0)`
in the child process immediately after fork and before exec. The process group ID
(PGID) SHALL equal the child's PID.

#### Scenario: New process group created on spawn

- **WHEN** a supervised process is spawned via `Command::spawn()`
- **THEN** the child process calls `setpgid(0, 0)` before exec
- **THEN** the returned `SpawnedProcess` includes a `pgid` field equal to the PID

#### Scenario: PGID equals PID

- **WHEN** a process is spawned successfully
- **THEN** `SpawnedProcess.pgid.as_raw()` equals `SpawnedProcess.pid.as_raw()`

### Requirement: Shutdown signals target the entire process group

When sending a shutdown signal (SIGTERM or SIGKILL) to a supervised process,
the signal SHALL be sent to the entire process group using `kill(-pgid, signal)`
instead of `kill(pid, signal)`. This ensures all child processes in the group
receive the termination signal.

#### Scenario: SIGTERM sent to process group

- **WHEN** the supervisor receives a shutdown event while the process is running
- **THEN** SIGTERM is sent to the entire process group via `kill(-pgid, SIGTERM)`

#### Scenario: SIGKILL escalation sent to process group

- **WHEN** the grace period expires and the process has not been reaped
- **THEN** SIGKILL is sent to the entire process group via `kill(-pgid, SIGKILL)`

#### Scenario: Signal to non-existent process group is handled gracefully

- **WHEN** `kill(-pgid, signal)` is called but the process group no longer exists
- **THEN** the error is logged at warn level and shutdown continues
