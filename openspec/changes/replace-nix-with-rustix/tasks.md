## Migration Strategy

### Keep nix for (NOT available in rustix 1.x):

- `nix::sys::signalfd::SignalFd` — listed in `rustix::not_implemented::yet`
- `nix::sys::signal::sigprocmask()` — not in rustix
- `nix::sys::signal::sigaction()` — not in rustix
- `nix::unistd::fork()` — not in rustix
- `nix::libc::_exit()` — use nix's re-exported libc, NOT standalone libc crate

### Migrate to rustix (available in rustix 1.x):

- epoll → `rustix::event::epoll`
- eventfd → `rustix::event::eventfd`
- splice → `rustix::fs::splice`
- pipe2 → `rustix::pipe::pipe2`
- dup2_stdout/dup2_stderr → `rustix::stdio`
- read/write → `rustix::io`
- close → `OwnedFd` drop
- getpid/getuid/getgid → `rustix::process`
- waitpid → `rustix::process::waitpid`
- mount → `rustix::mount`
- unshare → `rustix::thread`
- fsync/open → `rustix::fs`

---

## Tasks

### 1. Workspace dependency update

- [ ] 1.1 Update `rustix` to version 1.x with features: `event`, `fs`, `mount`, `pipe`, `process`, `thread`, `stdio`
- [ ] 1.2 Keep `nix` with features: `signal`, `fs`, `process`, `mount`, `sched`
- [ ] 1.3 DO NOT add standalone `libc` — use `nix::libc` instead
- [ ] 1.4 Update all crate `Cargo.toml` files

### 2. Signal handling — keep nix

- [x] 2.1 Keep `nix::sys::signal::sigprocmask()` — NOT in rustix
- [x] 2.2 Keep `nix::sys::signal::sigaction()` — NOT in rustix

### 3. Signalfd — keep nix

- [x] 3.1 Keep `nix::sys::signalfd::SignalFd` — NOT in rustix

### 4. Fork — keep nix

- [x] 4.1 Keep `nix::unistd::fork()` — NOT in rustix

### 5. Exit — use nix::libc

- [ ] 5.1 Replace `nix::libc::_exit()` calls — keep using nix::libc (verify it's nix::libc, not standalone)

### 6. Epoll migration to rustix

- [ ] 6.1 Replace `nix::sys::epoll` with `rustix::event::epoll` in zombie crate
- [ ] 6.2 Replace `nix::sys::epoll` with `rustix::event::epoll` in entry crate
- [ ] 6.3 Replace `nix::sys::epoll` with `rustix::event::epoll` in supervise splice_relay

### 7. Eventfd migration to rustix

- [ ] 7.1 Replace `nix::sys::eventfd::EventFd` with `rustix::event::eventfd` in waker.rs

### 8. Splice migration to rustix

- [ ] 8.1 Replace `nix::fcntl::splice()` with `rustix::fs::splice()` in entry lib.rs
- [ ] 8.2 Replace `nix::fcntl::splice()` with `rustix::fs::splice()` in supervise splice_relay

### 9. Pipe migration to rustix

- [ ] 9.1 Replace `nix::unistd::pipe2()` with `rustix::pipe::pipe2()` in entry, supervise

### 10. Stdio migration to rustix

- [ ] 10.1 Replace `nix::unistd::dup2_stdout()` with `rustix::stdio::dup2_stdout()`
- [ ] 10.2 Replace `nix::unistd::dup2_stderr()` with `rustix::stdio::dup2_stderr()`

### 11. I/O migration to rustix

- [ ] 11.1 Replace `nix::unistd::read()`/`write()` with `rustix::io::read()`/`write()`

### 12. Process info migration to rustix

- [ ] 12.1 Replace `nix::unistd::getpid()` with `rustix::process::getpid()`
- [ ] 12.2 Replace `nix::unistd::getuid()`/`getgid()` with `rustix::process::getuid()`/`getgid()`

### 13. Waitpid migration to rustix

- [ ] 13.1 Replace `nix::unistd::waitpid()` with `rustix::process::waitpid()` (returns `(Pid, WaitStatus)`)

### 14. Mount/unshare migration to rustix (test-utils)

- [ ] 14.1 Replace `nix::sched::unshare()` with `rustix::thread::unshare()`
- [ ] 14.2 Replace `nix::mount::mount()` with `rustix::mount::mount()`

### 15. File operations migration to rustix

- [ ] 15.1 Replace `nix::unistd::fsync()` with `rustix::fs::fsync()`
- [ ] 15.2 Replace `nix::fcntl::open()` with `rustix::fs::open()`

### 16. Verification

- [ ] 16.1 Run `cargo check --workspace`
- [ ] 16.2 Run `cargo clippy-all`
- [ ] 16.3 Run `cargo fmt --all --check`
- [ ] 16.4 Run `cargo nextest-all`
- [ ] 16.5 Verify nix only used for: signalfd, sigprocmask, sigaction, fork, nix::libc
