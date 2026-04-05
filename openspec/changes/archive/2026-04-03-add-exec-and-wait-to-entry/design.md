## Context

The bootstrap process has two modes:

1. **Supervise mode** (default): Runs `ocelot_entry::execute()` which forks and waits for a supervised process
2. **Shell mode**: Uses `switch_root_shell()` which does `execv` to replace itself with an interactive shell

Currently in shell mode, once `execv` succeeds, the original process is gone - there's no way to run cleanup code after the shell exits.

## Goals / Non-Goals

**Goals:**

- Extract supervisor loop from `execute()` into reusable `exec_and_wait()` function
- Add `execute_interactive()` that spawns shell with terminal setup and returns exit code
- Enable graceful shutdown after shell exits in bootstrap

**Non-Goals:**

- Not changing supervise mode behavior
- Not adding new features beyond what's needed for shell → shutdown flow

## Decisions

### 1. Extract `exec_and_wait()` function

**Decision**: Extract lines 142-209 from `execute()` into a new public function.

**Rationale**: This allows both `execute()` (non-interactive) and new `execute_interactive()` (interactive) to share the same supervisor loop. The function takes pid, stdout_fd, stderr_fd, and timeout.

**Alternatives considered**:

- Keep everything in `execute()` with mode flags: Rejected - becomes a kitchen sink
- Separate implementations: Rejected - duplicates ~50 lines of identical code

### 2. Shell config import in entry crate

**Decision**: Add a `ShellConfig` struct in entry or accept program + args directly.

**Rationale**: Entry currently has no concept of shell. Bootstrap has `ShellConfig` in its own crate. We could:

- Add shell config to entry (more coupling)
- Pass program/args directly to `execute_interactive()` (simpler)

**Chosen**: Pass `program: &str` and `args: &[&str]` directly - avoids pulling bootstrap's config types into entry.

### 3. Terminal setup location

**Decision**: Terminal setup happens in the child process after fork, before exec.

**Rationale**: `setsid` and `TIOCSCTTY` must be called in the process that will become the session leader. This must be in the forked child before `execv`.

## Risks / Trade-offs

- **[Risk]** Breaking existing `execute()` behavior during refactor
  - **Mitigation**: `execute()` should remain unchanged - just call `Process::spawn()` then `exec_and_wait()`
- **[Risk]** Stdin/stdout not available for I/O forwarding in interactive mode
  - **Mitigation**: Interactive mode doesn't need I/O forwarding - the shell takes over the terminal directly. The epoll loop can skip stdout/stderr tokens or pass dummy fds.

- **[Risk]** How to trigger shutdown in bootstrap after shell exits
  - **Mitigation**: `execute_interactive()` returns exit code. `switch_root_shell()` checks return value and calls shutdown function.
