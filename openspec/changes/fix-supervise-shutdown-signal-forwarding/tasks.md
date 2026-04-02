## 1. Process group support in spawned_process.rs

- [x] 1.1 Add `pgid: Pid` field to `SpawnedProcess` struct
- [x] 1.2 Call `setpgid(0, 0)` in child process after fork, before exec
- [x] 1.3 Return pgid (equal to pid) in parent's `SpawnedProcess`
- [x] 1.4 Update unit tests to verify pgid == pid

## 2. State tracks pgid alongside pid

- [x] 2.1 Add `pgid: Option<Pid>` field to `State` struct
- [x] 2.2 Update `set_running` to accept and store pgid
- [x] 2.3 Add `process_group_id()` method to State
- [x] 2.4 Update `set_exited` and `set_failed` to clear pgid

## 3. Signal entire process group in supervisor executor

- [x] 3.1 Replace `forward_signal(pid, signal)` with `forward_signal_group(pgid, signal)` using `kill(-pgid, signal)`
- [x] 3.2 Update all call sites to use pgid from state
- [x] 3.3 Add SAFETY comment and expect attribute for unsafe kill with negative PID

## 4. Active SIGKILL escalation via spawned task

- [x] 4.1 After sending SIGTERM, spawn a grace-period timer task that sends SIGKILL to the process group
- [x] 4.2 The timer task is cancelled when the process is reaped (via a shared flag or channel)
- [x] 4.3 Remove the dead deadline-check code at lines 173-179 in executor.rs
- [x] 4.4 Handle the `(Event::Shutdown, Phase::ShuttingDown)` case: already sends SIGKILL, no change needed

## 5. Reaper actively kills overdue processes during shutdown

- [x] 5.1 Track per-process kill deadline (registered_at + grace_period) in shutdown phase
- [x] 5.2 Before each SIGCHLD wait iteration, check for overdue processes and send SIGKILL
- [x] 5.3 Remove overdue processes from the registered set after killing them
- [x] 5.4 Continue waiting only for processes that haven't been killed yet

## 6. Verification

- [x] 6.1 Run `cargo check` to verify compilation
- [x] 6.2 Run `cargo test` to verify existing tests pass
- [x] 6.3 Run `cargo clippy-all` to verify no lint warnings
- [x] 6.4 Run `cargo fmt --all --check` to verify formatting
