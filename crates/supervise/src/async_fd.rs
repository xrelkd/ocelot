use std::{io, os::fd::OwnedFd};

use tokio::io::unix::AsyncFd;

/// Registers an owned file descriptor with the Tokio reactor.
///
/// # Errors
///
/// Returns the registration error if the descriptor cannot be registered.
pub fn register(fd: OwnedFd) -> io::Result<AsyncFd<OwnedFd>> {
    #[expect(
        unsafe_code,
        reason = "`OwnedFd` keeps the descriptor valid, open, and unchanged while registered."
    )]
    unsafe { AsyncFd::register(fd) }.map_err(io::Error::from)
}
