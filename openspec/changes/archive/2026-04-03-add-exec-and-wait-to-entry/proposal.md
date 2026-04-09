## Why

The bootstrap process currently uses `execv` to replace itself with an interactive shell in `switch_root_shell()`, which prevents any cleanup after the shell exits. To enable graceful system shutdown after the interactive shell terminates, we need a way to spawn the shell as a child process, wait for it to exit, and then trigger shutdown. The `ocelot_entry` crate already has the machinery for process supervision but lacks interactive shell support.

## What Changes

- Refactor `ocelot_entry` to extract the core supervisor loop into a new `exec_and_wait()` function
- Add `execute_interactive()` function that sets up terminal (setsid, TIOCSCTTY), forks and execs shell, then waits for exit
- Modify `switch_root_shell()` in bootstrap to use `execute_interactive()` and trigger shutdown on return
- Both entry points return exit code to enable post-shell cleanup

## Capabilities

### New Capabilities

- `exec-and-wait`: Extracted supervisor loop reusable for both non-interactive and interactive modes
- `execute-interactive`: Spawn interactive shell with terminal setup, return exit code on completion

### Modified Capabilities

- `bootstrap-shell`: Now uses entry's `execute_interactive()` instead of direct execv, enabling post-exit shutdown

## Impact

- `crates/entry/src/lib.rs`: Refactor to extract `exec_and_wait()`, add `execute_interactive()`
- `crates/bootstrap/src/switch_root.rs`: Use entry's function, add shutdown call after shell exits
- No new dependencies required
