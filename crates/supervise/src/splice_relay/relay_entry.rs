use std::os::unix::io::OwnedFd;

use tokio::sync::oneshot;

use crate::splice_relay::Destination;

#[derive(Debug)]
pub struct RelayEntry {
    pub id: u64,
    pub source: OwnedFd,
    pub destination: Destination,
    pub start_notification: Option<oneshot::Sender<()>>,
}

impl RelayEntry {
    #[must_use]
    pub const fn new(
        id: u64,
        source: OwnedFd,
        destination: Destination,
        start_notification: Option<oneshot::Sender<()>>,
    ) -> Self {
        Self { id, source, destination, start_notification }
    }
}
