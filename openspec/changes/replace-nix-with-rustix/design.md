## Context

The ocelot project currently depends on `nix` v0.31 as its sole interface to Linux syscalls. All 6 workspace crates use `nix` for signal handling, process management, epoll-based I/O multiplexing, zero-copy splice, eventfd, mount, and unshare operations.

**Key Finding:** `rustix` 1.x does NOT provide these APIs:

- `signalfd` — listed in `rustix::not_implemented::yet`
- `sigprocmask` — not available in rustix
- `sigaction` — not available in rustix
- `fork` — not available in rustix

**Strategy:** Mixed approach — use rustix where APIs exist, keep nix for missing APIs. Use `nix::libc` (NOT standalone libc) for `_exit()` calls.

## Goals / Non-Goals

**Goals:**

- Migrate as many syscalls as possible from nix to rustix
- Keep nix only for: signalfd, sigprocmask, sigaction, fork
- Use `nix::libc` for `_exit()` (do NOT add standalone libc dependency)
- Maintain identical runtime behavior
- No breaking changes to public API

**Non-Goals:**

- No refactoring of existing architecture or logic
- No changes to public configuration formats or CLI behavior
- No performance optimization beyond what rustix naturally provides

## Decisions

### 1. Mixed approach: rustix + nix

**Decision:** Use both `rustix` and `nix` crates. Rustix for available APIs, nix for missing APIs.

**Keep nix for:**

- `nix::sys::signalfd` — not_implemented::yet in rustix
- `nix::sys::signal::sigprocmask` — not in rustix
- `nix::sys::signal::sigaction` — not in rustix
- `nix::unistd::fork` — not in rustix
- `nix::libc::_exit` — use nix's re-exported libc

**Migrate to rustix:**

- epoll, eventfd, splice, pipe2, dup2_stdout/dup2_stderr
- read/write, close, getpid, getuid/getgid
- waitpid, mount, unshare, fsync, open, stat

### 2. Feature flags

```toml
[workspace.dependencies]
nix = { version = "0.31", features = ["signal", "fs", "process", "mount", "sched"] }
rustix = { version = "1", features = ["event", "fs", "mount", "pipe", "process", "thread", "stdio"] }
# NO standalone libc — use nix::libc
```

### 3. libc usage — use nix::libc only

**Decision:** Use `nix::libc` for all libc types/functions. Do NOT add standalone `libc` crate.

```rust
// Correct:
nix::libc::_exit(0)

// Incorrect:
use libc::_exit;  // Do NOT do this
```

**Rationale:** nix already depends on libc internally. Using `nix::libc` avoids adding another dependency and ensures version compatibility.

### 4. Signal handling — keep nix

- `nix::sys::signal::sigprocmask()` — keep nix (not in rustix)
- `nix::sys::signal::sigaction()` — keep nix (not in rustix)
- `nix::sys::signal::kill()` → `rustix::process::kill()`
- `nix::sys::signal::Signal` → `rustix::process::Signal` where compatible

### 5. Signalfd — keep nix

- `nix::sys::signalfd::SignalFd` — keep nix (not_implemented::yet in rustix)

### 6. Process management

- `nix::unistd::fork()` — keep nix (not in rustix)
- `nix::unistd::waitpid()` → `rustix::process::waitpid()` (returns `(Pid, WaitStatus)`)
- `nix::unistd::getpid()` → `rustix::process::getpid()`
- `nix::unistd::getuid()`/`getgid()` → `rustix::process::getuid()`/`getgid()`

### 7. Pipe and file descriptor — migrate to rustix

- `nix::unistd::pipe2()` → `rustix::pipe::pipe2()`
- `nix::unistd::dup2_stdout()`/`dup2_stderr()` → `rustix::stdio::dup2_stdout()`/`dup2_stderr()`
- `nix::unistd::read()`/`write()` → `rustix::io::read()`/`write()`
- `nix::unistd::close()` → `OwnedFd` drop pattern

### 8. Epoll — migrate to rustix

- `nix::sys::epoll::Epoll` → `rustix::event::epoll` functions
- `epoll_create1()`, `epoll_ctl()`, `wait()` pattern

### 9. Eventfd — migrate to rustix

- `nix::sys::eventfd::EventFd` → `rustix::event::eventfd::eventfd()`

### 10. Splice — migrate to rustix

- `nix::fcntl::splice()` → `rustix::fs::splice()`
- `nix::fcntl::SpliceFFlags` → `rustix::fs::SpliceFlags`

### 11. Mount and unshare — migrate to rustix

- `nix::mount::mount()` → `rustix::mount::mount()`
- `nix::sched::unshare()` → `rustix::thread::unshare()`

### 12. File operations — migrate to rustix

- `nix::unistd::fsync()` → `rustix::fs::fsync()`
- `nix::fcntl::open()` → `rustix::fs::open()`

## Migration Order

1. Update workspace Cargo.toml (update rustix to 1.x, ensure nix has correct features)
2. Migrate zombie crate (uses epoll)
3. Migrate entry crate (uses epoll, splice, pipe2, dup2)
4. Migrate idle crate (uses epoll)
5. Migrate supervise crate (uses eventfd, epoll, splice)
6. Update test-utils crate (uses mount, unshare)
7. Verify all tests pass
