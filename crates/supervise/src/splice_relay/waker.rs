use std::{mem::MaybeUninit, sync::Arc};

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
    const WAKE_BYTES: [u8; 8] = 0xC0DE_CAFE_C0DE_CAFEu64.to_ne_bytes();

    pub fn new() -> Result<Self, Error> {
        let event_fd =
            EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK | EfdFlags::EFD_CLOEXEC)
                .with_context(|_| error::CreateEventfdSnafu)?;
        Ok(Self::from(Arc::new(event_fd)))
    }

    #[inline]
    pub fn wake(&self) { let _ = unistd::write(self.event_fd.as_ref(), &Self::WAKE_BYTES); }

    #[inline]
    pub fn consume(&self) {
        let mut buf = MaybeUninit::<u64>::uninit();

        #[expect(
            unsafe_code,
            reason = "The kernel will initialize the buffer memory via a system call before it is \
                      accessed by Rust code."
        )]
        let slice = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), 8) };
        let _ = unistd::read(self.as_ref(), slice);
        debug_assert_eq!(slice, Self::WAKE_BYTES);
    }
}
