## MODIFIED Requirements

### Requirement: Supervisor escalates to SIGKILL after grace period expires

The supervisor executor SHALL actively escalate to SIGKILL after the
`termination_grace_period` elapses following a SIGTERM. This escalation SHALL
be implemented via a spawned async task that sleeps for the grace period and
then sends SIGKILL to the process group if the process has not been reaped.
The previous dead-code deadline check at the bottom of the event loop SHALL
be removed.

#### Scenario: SIGKILL sent after grace period

- **WHEN** SIGTERM is sent and the grace period elapses without the process being reaped
- **THEN** SIGKILL is sent to the process group

#### Scenario: SIGKILL not sent if process exits in time

- **WHEN** SIGTERM is sent and the process exits before the grace period elapses
- **THEN** no SIGKILL is sent

#### Scenario: Shutdown from Running phase exits event loop

- **WHEN** Event::Shutdown is received while in Phase::Running
- **THEN** SIGTERM is sent to the process group, state is set to ShuttingDown,
  a grace-period escalation task is spawned, and the event loop breaks

#### Scenario: Shutdown from ShuttingDown phase sends SIGKILL immediately

- **WHEN** Event::Shutdown is received while already in Phase::ShuttingDown
- **THEN** SIGKILL is sent to the process group and the event loop breaks

### Requirement: Reaper actively kills overdue processes during shutdown

During its shutdown phase, the reaper SHALL check each registered process's
individual grace period deadline and actively send SIGKILL to processes whose
deadline has elapsed, rather than passively waiting for the maximum grace
period to expire.

#### Scenario: Reaper SIGKILLs process exceeding its grace period

- **WHEN** the reaper enters shutdown and a registered process has exceeded its
  termination_grace_period
- **THEN** the reaper sends SIGKILL to that process's process group

#### Scenario: Reaper waits only as long as needed

- **WHEN** all registered processes have exited or been killed
- **THEN** the reaper exits its shutdown phase immediately without waiting for
  the maximum grace period
