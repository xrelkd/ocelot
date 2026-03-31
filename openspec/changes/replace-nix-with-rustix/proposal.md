## Why

The `nix` crate is a high-level wrapper that adds abstraction overhead and limits access to newer Linux syscalls. `rustix` provides a more direct, zero-cost abstraction over Linux syscalls with better performance, active maintenance, and finer-grained feature control.

**Key Finding:** `rustix` 1.x does NOT provide all APIs we need (`signalfd`, `sigprocmask`, `sigaction`, `fork`). The migration strategy is a **mixed approach**: use rustix where APIs exist, keep nix for missing APIs.

## What Changes

- Update `rustix` to version 1.x with features: `event`, `fs`, `mount`, `pipe`, `process`, `thread`, `stdio`
- Keep `nix` (v0.31) with features: `signal`, `fs`, `process`, `mount`, `sched` for missing APIs
- Use `nix::libc` (not standalone `libc`) for `_exit()` calls
- Migrate epoll from `nix::sys::epoll` to `rustix::event::epoll`
- Migrate eventfd from `nix::sys::eventfd` to `rustix::event::eventfd`
- Migrate splice from `nix::fcntl::splice` to `rustix::fs::splice`
- Migrate pipe2 from `nix::unistd::pipe2` to `rustix::pipe::pipe2`
- Migrate dup2_stdout/dup2_stderr to `rustix::stdio`
- Migrate read/write to `rustix::io`
- Migrate getpid/getuid/getgid to `rustix::process`
- Migrate waitpid to `rustix::process::waitpid` (different API)
- Migrate mount/unshare to `rustix::mount`/`rustix::thread`
- Migrate fsync/open to `rustix::fs`
- **Keep nix for:** signalfd, sigprocmask, sigaction, fork
- **No public API changes** — internal implementation only

## Capabilities

### New Capabilities

- `rustix-syscall-bindings`: Direct Linux syscall bindings via rustix for epoll, eventfd, splice, pipe, stdio, process info, mount, and thread operations

### Modified Capabilities

- Signal handling: Uses nix for sigprocmask/sigaction/signalfd (not available in rustix)
- Process forking: Uses nix for fork (not available in rustix)

## Impact

- **Dependencies**: Both `nix` (for missing APIs) and `rustix` 1.x coexist; uses `nix::libc` for `_exit()` calls
- **Affected crates**: All 6 workspace members (`ocelot`, `ocelot-entry`, `ocelot-idle`, `ocelot-supervise`, `ocelot-test-utils`, `ocelot-zombie`)
- **Unsafe code**: `fork()` remains on nix with existing SAFETY comments
- **Tests**: Integration tests using signal types may need updates
- **Public API**: No breaking changes
