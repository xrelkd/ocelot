use snafu::Snafu;

/// Errors that can occur during zombie process generation.
///
/// This enum covers system call failures, signal handling issues, and
/// resource conversion errors encountered while spawning and monitoring
/// zombie processes.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Failed to set the signal mask.
    ///
    /// This error occurs when `sigprocmask()` fails to block signals.
    #[snafu(display("Failed to set signal mask, error: {source}"))]
    SetSignalMask { source: nix::errno::Errno },

    /// Failed to create a signal file descriptor.
    ///
    /// This error occurs when `signalfd()` fails to create a file descriptor
    /// for receiving signals.
    #[snafu(display("Failed to create signal fd, error: {source}"))]
    CreateSignalFd { source: nix::errno::Errno },

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

    #[snafu(display("Failed to spawn child process, error: {source}"))]
    SpawnChild { source: nix::Error },

    /// Failed to create epoll instance.
    ///
    /// This error occurs when `Epoll::new()` fails to create an epoll file
    /// descriptor.
    #[snafu(display("Failed to create epoll, error: {source}"))]
    CreateEpoll { source: nix::Error },

    /// Failed to register file descriptor with epoll.
    ///
    /// This error occurs when `epoll::ctl()` fails to add or modify a file
    /// descriptor.
    #[snafu(display("Failed to register fd with epoll, error: {source}"))]
    AddEpoll { source: nix::Error },

    /// Failed to wait for epoll events.
    ///
    /// This error occurs when `epoll::wait()` fails, which may be due to
    /// interruption or other system errors.
    #[snafu(display("Failed to wait for epoll events, error: {source}"))]
    WaitEpoll { source: nix::Error },

    /// Failed to convert duration to epoll timeout.
    ///
    /// This error occurs when converting milliseconds to `PollTimeout` fails
    /// due to integer overflow.
    #[snafu(display("Failed to convert duration to timeout: {source}"))]
    CreateTimeout { source: nix::poll::PollTimeoutTryFromError },
}
