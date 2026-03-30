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

#[derive(Debug, Clone)]
pub struct RelayInfo {
    pub id: u64,
}

#[derive(Debug)]
pub enum Event {
    Register {
        source: OwnedFd,
        destination: Destination,
        sender: oneshot::Sender<Option<u64>>,
        start_notification: Option<oneshot::Sender<()>>,
    },
    RemoveRelay {
        id: u64,
    },
    GetStatus {
        sender: oneshot::Sender<Status>,
    },
    ListRelays {
        sender: oneshot::Sender<Vec<RelayInfo>>,
    },
    Shutdown,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub active_relays: usize,
    pub total_added: u64,
    pub total_removed: u64,
    pub bytes_transferred: u64,
}
