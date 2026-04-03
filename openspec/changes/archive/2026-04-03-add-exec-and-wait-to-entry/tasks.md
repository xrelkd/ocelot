## 1. Refactor Entry Crate

- [x] 1.1 Extract supervisor loop from `execute()` into new `exec_and_wait()` function in `crates/entry/src/lib.rs`
- [x] 1.2 Refactor `execute()` to call `Process::spawn()` then `exec_and_wait()`
- [x] 1.3 Verify existing tests still pass after refactor

## 2. Add execute_interactive Function

- [x] 2.1 Create `execute_interactive(console, program, args, timeout)` function in `crates/entry/src/lib.rs`
- [x] 2.2 Implement terminal setup (console open, setsid, dup2, TIOCSCTTY)
- [x] 2.3 Implement fork + exec shell pattern
- [x] 2.4 Call `exec_and_wait()` in parent to wait for shell exit
- [x] 2.5 Return exit code on completion

## 3. Update Bootstrap to Use execute_interactive

- [x] 3.1 Add dependency on `ocelot_entry` in `crates/bootstrap/Cargo.toml` (if not already)
- [x] 3.2 Refactor `switch_root_shell()` to use `execute_interactive()` from entry
- [x] 3.3 Add shutdown call after `execute_interactive()` returns
- [x] 3.4 Handle error cases appropriately

## 4. Verify and Test

- [x] 4.1 Run `cargo build` to verify compilation
- [x] 4.2 Run `cargo clippy-all` for lint checks
- [x] 4.3 Run tests to ensure no regressions
