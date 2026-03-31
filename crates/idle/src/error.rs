use snafu::Snafu;

/// Errors that can occur when setting up the idle process supervisor.
///
/// The idle process handles signals and reaps zombies. This error type
/// represents failures during initialization and signal handling.
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
}
