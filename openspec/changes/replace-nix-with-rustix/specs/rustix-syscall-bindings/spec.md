## MODIFIED Requirements

### Requirement: Signal handling keeps nix (NOT available in rustix)

**Note:** rustix 1.x does NOT have `sigprocmask`, `sigaction`, or `signalfd`. These APIs remain on nix.

The system SHALL continue using `nix::sys::signal` for signal mask manipulation (`sigprocmask`) and signal handler registration (`sigaction`).

#### Scenario: Block signals in parent thread

- **WHEN** the system sets up signal handling
- **THEN** `nix::sys::signal::sigprocmask()` is used to block target signals

#### Scenario: Register signal handler

- **WHEN** a signal handler needs to be registered
- **THEN** `nix::sys::signal::sigaction()` is used with appropriate `SaFlags`

### Requirement: Signalfd keeps nix (NOT available in rustix)

**Note:** `signalfd` is listed in `rustix::not_implemented::yet`. The system SHALL continue using `nix::sys::signalfd` for signal-as-file-descriptor patterns.

#### Scenario: Create signalfd

- **WHEN** the system needs to receive signals via a file descriptor
- **THEN** `nix::sys::signalfd::SignalFd::new()` is called with the signal mask

#### Scenario: Read signal info from signalfd

- **WHEN** the signalfd becomes readable
- **THEN** `signal_fd.read_signal()` is used to read `signalfd_siginfo` data

### Requirement: Fork keeps nix (NOT available in rustix)

The system SHALL continue using `nix::unistd::fork()` for process forking since rustix does not provide this API.

#### Scenario: Fork a child process

- **WHEN** the system spawns a new process
- **THEN** `nix::unistd::fork()` is called within an `unsafe` block with a SAFETY comment

### Requirement: Exit uses nix::libc (NOT standalone libc)

The system SHALL use `nix::libc::_exit()` for child process exit. Do NOT import standalone `libc` crate.

#### Scenario: Exit child process after fork

- **WHEN** a forked child needs to exit
- **THEN** `nix::libc::_exit(code)` is called within an `unsafe` block

---

## ADDED Requirements

### Requirement: Epoll uses rustix event API

The system SHALL use `rustix::event::epoll` for I/O event multiplexing, replacing `nix::sys::epoll`.

#### Scenario: Create epoll instance

- **WHEN** the system needs to multiplex multiple file descriptors
- **THEN** `rustix::event::epoll::epoll_create1()` is called

#### Scenario: Register file descriptor with epoll

- **WHEN** a file descriptor needs to be monitored
- **THEN** `rustix::event::epoll::epoll_ctl()` with `EpollOp::Add` is called

#### Scenario: Wait for epoll events

- **WHEN** the system polls for ready file descriptors
- **THEN** `rustix::event::epoll::wait()` is called with a pre-allocated event slice

#### Scenario: Remove file descriptor from epoll

- **WHEN** a monitored file descriptor is no longer needed
- **THEN** `rustix::event::epoll::epoll_ctl()` with `EpollOp::Delete` is called

### Requirement: Eventfd uses rustix event API

The system SHALL use `rustix::event::eventfd` for event notification file descriptors, replacing `nix::sys::eventfd`.

#### Scenario: Create eventfd for waker

- **WHEN** the system needs an eventfd to wake an epoll loop
- **THEN** `rustix::event::eventfd::eventfd()` is called with initial value and flags

### Requirement: Splice uses rustix fs API

The system SHALL use `rustix::fs::splice` for zero-copy data transfer between pipes, replacing `nix::fcntl::splice`.

#### Scenario: Relay data between pipes via splice

- **WHEN** the system forwards data from a source pipe to a destination pipe
- **THEN** `rustix::fs::splice()` is called with source fd, destination fd, byte count, and flags

### Requirement: Pipe operations use rustix pipe API

The system SHALL use `rustix::pipe::pipe2()` for creating pipes with flags, replacing `nix::unistd::pipe2()`.

#### Scenario: Create non-blocking pipe

- **WHEN** the system needs a non-blocking pipe for child I/O
- **THEN** `rustix::pipe::pipe2()` is called with `PipeFlags::NONBLOCK`

#### Scenario: Create close-on-exec pipe

- **WHEN** the system needs a pipe that closes on exec
- **THEN** `rustix::pipe::pipe2()` is called with `PipeFlags::CLOEXEC`

### Requirement: File descriptor operations use rustix stdio and io APIs

The system SHALL use `rustix::stdio` and `rustix::io` for file descriptor operations.

#### Scenario: Duplicate file descriptor to stdout

- **WHEN** the system redirects child stdout
- **THEN** `rustix::stdio::dup2_stdout()` is called

#### Scenario: Duplicate file descriptor to stderr

- **WHEN** the system redirects child stderr
- **THEN** `rustix::stdio::dup2_stderr()` is called

#### Scenario: Read from file descriptor

- **WHEN** the system reads data from a pipe or fd
- **THEN** `rustix::io::read()` is called

#### Scenario: Write to file descriptor

- **WHEN** the system writes data to a pipe or fd
- **THEN** `rustix::io::write()` is called

### Requirement: Process info uses rustix process API

The system SHALL use `rustix::process` for process information queries.

#### Scenario: Get current process ID

- **WHEN** the system needs to know its own PID
- **THEN** `rustix::process::getpid()` is called

#### Scenario: Get real user ID

- **WHEN** the system needs the real user ID
- **THEN** `rustix::process::getuid()` is called

#### Scenario: Get real group ID

- **WHEN** the system needs the real group ID
- **THEN** `rustix::process::getgid()` is called

### Requirement: Waitpid uses rustix process API

The system SHALL use `rustix::process::waitpid()` for child process reaping where possible.

#### Scenario: Wait for child process (non-blocking)

- **WHEN** the system checks if a child has exited
- **THEN** `rustix::process::waitpid()` is called with `WaitOptions::NOHANG`

### Requirement: Mount operations use rustix mount API

The system SHALL use `rustix::mount` for filesystem mount operations.

#### Scenario: Mount proc filesystem

- **WHEN** test utilities set up an isolated environment
- **THEN** `rustix::mount::mount()` is called with appropriate `MountFlags`

### Requirement: Namespace unshare uses rustix thread API

The system SHALL use `rustix::thread::unshare()` for namespace isolation.

#### Scenario: Create new user namespace

- **WHEN** test utilities need user namespace isolation
- **THEN** `rustix::thread::unshare()` is called with `UnshareFlags::NEWUSER`

### Requirement: File operations use rustix fs API

The system SHALL use `rustix::fs` for file operations.

#### Scenario: Sync file to disk

- **WHEN** log rotation requires durability
- **THEN** `rustix::fs::fsync()` is called with the file descriptor

#### Scenario: Open file with flags

- **WHEN** the system needs to open /dev/null or other files
- **THEN** `rustix::fs::open()` is called with appropriate flags and mode
