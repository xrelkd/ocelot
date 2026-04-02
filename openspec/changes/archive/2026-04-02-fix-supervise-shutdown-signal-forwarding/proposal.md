## Why

When ocelot supervise manages sshd (or any multi-process daemon), sending SIGINT
to ocelot causes sshd to hang for an extended period before exiting. This happens
because:

1. The supervisor sends SIGTERM to the direct child process only, not to its
   entire process tree. sshd spawns session child processes that never receive
   the termination signal.
2. The grace-period SIGKILL escalation logic in the supervisor event loop is
   dead code — the loop breaks immediately after sending SIGTERM, so the
   deadline check never runs.
3. The reaper's shutdown phase passively waits for processes to exit but never
   actively SIGKILLs stragglers that exceed their grace period.
4. Child processes are spawned in the same process group as ocelot, so there is
   no way to signal the entire process tree atomically.

The result is that ocelot waits up to the full `shutdown_timeout` (default 30s)
for sshd to exit, and sshd itself waits indefinitely for its session children
to terminate.

## What Changes

- **Supervisor Executor**: After sending SIGTERM, spawn a grace-period timer
  task that actively sends SIGKILL if the process has not been reaped. Remove
  the dead deadline-check code at the bottom of the event loop.
- **Process Group Isolation**: On spawn, call `setpgid(0, 0)` so each supervised
  process gets its own process group. On shutdown, signal the entire group with
  `kill(-pgid, signal)` instead of just the leader PID.
- **Reaper Active Kill**: During shutdown, the reaper will actively SIGKILL
  registered processes whose grace period has elapsed, instead of passively
  waiting for the maximum grace period to expire.
- **SpawnedProcess pgid tracking**: The `SpawnedProcess` struct gains a `pgid`
  field so the supervisor knows which process group to signal.

## Capabilities

### New Capabilities

- `process-group-signaling`: Supervised processes are placed in their own
  process group via `setpgid(0, 0)` after fork. Shutdown signals are sent to
  the entire process group (`kill(-pgid, signal)`) so that child processes
  (e.g., sshd sessions) also receive termination signals.

### Modified Capabilities

- `supervisor-shutdown`: The supervisor executor now actively escalates to
  SIGKILL after the grace period expires, instead of relying on dead code
  that was never reachable. The reaper also actively kills overdue processes
  during its shutdown phase.

## Impact

- `crates/supervise/src/supervisor/executor.rs`: Core shutdown logic rewrite —
  grace-period timer task replaces dead deadline check.
- `crates/supervise/src/supervisor/spawned_process.rs`: Add `pgid` field to
  `SpawnedProcess`; call `setpgid(0, 0)` in child after fork.
- `crates/supervise/src/supervisor/state.rs`: Track pgid alongside pid.
- `crates/supervise/src/reaper/executor.rs`: Active SIGKILL during shutdown
  for processes exceeding grace period.
- Signal forwarding now targets process groups, which is a behavioral change
  for any supervised process that has children.
