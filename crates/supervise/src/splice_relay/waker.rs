use std::sync::Arc;

use nix::{
    sys::eventfd::{EfdFlags, EventFd},
    unistd,
};
use snafu::ResultExt;

use crate::splice_relay::{Error, error};

#[derive(Clone, Debug)]
pub struct Waker {
    event_fd: Arc<EventFd>,
}

impl AsRef<EventFd> for Waker {
    fn as_ref(&self) -> &EventFd { self.event_fd.as_ref() }
}

impl From<Arc<EventFd>> for Waker {
    fn from(event_fd: Arc<EventFd>) -> Self { Self { event_fd } }
}

impl Waker {
    const WAKE_MAGIC_NUMBER: u64 = 0xcafe_cafe;

    pub fn new() -> Result<Self, Error> {
        let event_fd =
            EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK | EfdFlags::EFD_CLOEXEC)
                .with_context(|_| error::CreateEventfdSnafu)?;
        Ok(Self::from(Arc::new(event_fd)))
    }

    #[inline]
    pub fn wake(&self) {
        let data = const { Self::WAKE_MAGIC_NUMBER.to_ne_bytes() };
        let _ = unistd::write(self.event_fd.as_ref(), &data);
    }
}
