use snafu::Snafu;

/// Errors that can occur when spawning and managing a child process.
///
/// This enum covers all failure modes including invalid input, system call
/// errors, and child process execution failures.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Invalid command or argument containing interior null bytes.
    ///
    /// This error occurs when the command or any argument cannot be converted
    /// to a C string because it contains null bytes (`\0`).
    InvalidInput { input: String, source: std::ffi::NulError },

    /// Failed to fork the child process.
    ///
    /// This error indicates that the `fork()` system call failed, which could
    /// be due to resource limits (PID, memory) or permission issues.
    #[snafu(display("Failed to spawn child process, error: {source}"))]
    SpawnChild { source: nix::Error },

    /// Failed to wait for a child process.
    ///
    /// This error occurs when calling `waitpid()` to reap a zombie process
    /// or obtain the exit status.
    #[snafu(display("Failed to wait for child process (nix), error: {source}"))]
    WaitPid { source: nix::Error },

    /// Child process failed to execute the command.
    ///
    /// This error is returned when the child process successfully forks but
    /// `execvp()` fails to replace the process image. The original error from
    /// `execvp` is logged but not returned directly.
    #[snafu(display("Failed to execute child process"))]
    ExecuteChild,

    /// Failed to read from a pipe.
    ///
    /// This error indicates an I/O error when reading from a file descriptor,
    /// typically during the error pipe communication after fork.
    #[snafu(display("Failed to read from Pipe, error: {source}"))]
    ReadPipe { source: nix::Error },

    /// Failed to create a pipe.
    ///
    /// This error occurs when `pipe2()` fails to create a pipe file descriptor
    /// pair for communication between parent and child.
    #[snafu(display("Failed to construct Pipe, error: {source}"))]
    CreatePipe { source: nix::Error },

    /// Failed to set the signal mask.
    ///
    /// This error occurs when `sigprocmask()` fails to block or unblock
    /// signals.
    #[snafu(display("Failed to set signal mask, error: {source}"))]
    SetSignalMask { source: nix::errno::Errno },

    /// Failed to create a signal file descriptor.
    ///
    /// This error occurs when `signalfd()` fails to create a file descriptor
    /// for receiving signals.
    #[snafu(display("Failed to create signal fd, error: {source}"))]
    CreateSignalFd { source: nix::errno::Errno },

    /// Failed to create an epoll instance.
    ///
    /// This error indicates that `epoll_create1()` failed to allocate kernel
    /// resources for the event poll.
    #[snafu(display("Failed to create epoll instance, error: {source}"))]
    CreateEpoll { source: nix::Error },

    /// Failed to add a file descriptor to epoll.
    ///
    /// This error occurs when `epoll_ctl()` with `EPOLL_CTL_ADD` fails to
    /// register a file descriptor with the epoll instance.
    #[snafu(display("Failed to add fd to epoll: {source}"))]
    AddEpoll { source: nix::Error },

    /// Failed to convert a timeout to poll timeout.
    ///
    /// This error occurs when converting a `Duration` to `PollTimeout` for
    /// use with `epoll_wait()`.
    #[snafu(display("Failed to convert timeout to poll timeout: {source}"))]
    ConvertTimeout { source: nix::poll::PollTimeoutTryFromError },

    /// Failed to wait for epoll events.
    ///
    /// This error indicates that `epoll_wait()` failed, which could be due
    /// to an interrupted system call or other kernel error.
    #[snafu(display("Failed to wait for epoll events, error: {source}"))]
    WaitEpoll { source: nix::Error },

    /// Failed to parse signal number.
    ///
    /// This error occurs when reading a signal from the signal fd and the
    /// signal number cannot be converted to a valid `Signal` enum variant.
    #[snafu(display("Failed to parse signal number: {signal_num}"))]
    ParseSignal { signal_num: i32, source: nix::errno::Errno },

    /// Failed to convert u32 to i32.
    ///
    /// This error occurs when converting a signal number from `u32` (as
    /// provided by the kernel) to `i32` for use with the `Signal` enum.
    #[snafu(display("Failed to convert u32 to i32: {value}"))]
    ConvertU32 { value: u32, source: std::num::TryFromIntError },
}
