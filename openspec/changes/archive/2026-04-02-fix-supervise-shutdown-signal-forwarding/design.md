## Context

The supervise crate manages supervised processes through an event loop in
`SupervisorExecutor`. When SIGINT/SIGTERM arrives, the orchestrator sends
`Event::Shutdown` to each supervisor. The supervisor sends SIGTERM to the
direct child PID and breaks out of the event loop immediately.

Two structural problems cause sshd to hang:

1. **Dead SIGKILL escalation code**: Lines 173-179 in `executor.rs` check
   `shutdown_deadline_exceeded()` and send SIGKILL, but this code is
   unreachable because the `(Event::Shutdown, Phase::Running)` arm breaks
   before the check can ever run again.

2. **No process group signaling**: `fork()` does not call `setpgid(0, 0)`,
   so the child shares ocelot's process group. `kill(pid, signal)` only
   targets the leader — sshd session children never receive the signal.

3. **Reaper passive shutdown**: The reaper's shutdown phase waits up to the
   maximum grace period for all registered processes, but never actively
   SIGKILLs processes that have exceeded their individual grace periods.

## Goals / Non-Goals

**Goals:**

- Actively escalate to SIGKILL after grace period in supervisor executor
- Place each supervised process in its own process group via `setpgid(0, 0)`
- Signal the entire process group on shutdown via `kill(-pgid, signal)`
- Reaper actively SIGKILLs overdue processes during its shutdown phase
- Maintain backward compatibility — existing configs work unchanged

**Non-Goals:**

- No changes to the entry crate (single-process supervisor, already works)
- No changes to signal semantics for processes without children
- No new dependencies — uses nix `process` feature already in Cargo.toml

## Decisions

### 1. Grace-period timer as a spawned task (not inline loop check)

**Decision**: After sending SIGTERM, spawn a dedicated async task that sleeps
for `termination_grace_period` then sends SIGKILL if the process hasn't been
reaped. The task is cancelled automatically when the supervisor exits because
it runs under the same `JoinSet`.

**Rationale**: The current approach of checking `shutdown_deadline_exceeded()`
at the bottom of the event loop only works while the loop is running. Since
the shutdown arm breaks immediately, the check is dead code. A spawned task
runs independently and doesn't depend on the event loop continuing.

**Alternatives considered**:

- Keep the loop running with `tokio::time::sleep` — would require restructuring
  the entire event loop to handle timeouts, adding complexity.
- Use `tokio::time::timeout` around the entire shutdown sequence — would kill
  the whole supervisor executor, not just escalate the signal.

### 2. Process group via `setpgid(0, 0)` in child after fork

**Decision**: Call `setpgid(0, 0)` in the child process immediately after
`fork()` and before `exec()`. This creates a new process group with the child
as leader. The parent stores the `pgid` (equal to `pid`) alongside the PID.

**Rationale**: `setpgid(0, 0)` is the standard POSIX way to create a new
process group. Using the child's PID as the PGID means we can signal the
entire tree with `kill(-pgid, signal)`. This is simpler than managing
separate PGID allocation.

**Alternatives considered**:

- `setsid()` — creates a new session, which is heavier than needed and may
  interfere with terminal job control.
- Parent calls `setpgid(child, child)` — requires the child to not have
  already called `exec()`, which is a race condition.

### 3. Signal the process group with negative PID

**Decision**: Replace `forward_signal(pid, signal)` with
`forward_signal_group(pgid, signal)` that calls `kill(-pgid, signal)`. The
nix `kill()` function accepts negative PIDs to mean process groups.

**Rationale**: This is the standard POSIX mechanism for signaling a process
group. It ensures all children (sshd sessions, grand-children, etc.) receive
the signal simultaneously.

### 4. Reaper active kill during shutdown

**Decision**: In the reaper's shutdown phase, track each process's individual
grace period deadline. On each SIGCHLD iteration, also check for processes
whose deadline has passed and send them SIGKILL before waiting.

**Rationale**: The current approach waits for the maximum grace period
regardless of individual process deadlines. A process with a 5-second grace
period should not block shutdown for 30 seconds because another process has
a 30-second grace period.

## Risks / Trade-offs

| Risk                                                                          | Mitigation                                                                        |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Process group signaling may affect unrelated processes if pgid collides       | `setpgid(0, 0)` guarantees a unique pgid equal to the child's pid                 |
| Some processes may not handle process group signals correctly                 | This is standard POSIX behavior; sshd, nginx, etc. all handle it                  |
| SIGKILL escalation task may fire after process already exited                 | `kill()` on non-existent PID returns `ESRCH`, which is already logged and ignored |
| Breaking change for processes that rely on sharing the parent's process group | Documented behavioral change; supervised processes should be isolated anyway      |
