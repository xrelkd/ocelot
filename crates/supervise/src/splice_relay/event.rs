use std::os::unix::io::OwnedFd;

use tokio::sync::oneshot;

#[derive(Debug)]
pub struct RelayEntry {
    pub id: u64,
    pub source: OwnedFd,
    pub destination: OwnedFd,
}

impl RelayEntry {
    #[must_use]
    pub const fn new(id: u64, source: OwnedFd, destination: OwnedFd) -> Self {
        Self { id, source, destination }
    }
}

#[derive(Debug, Clone)]
pub struct RelayInfo {
    pub id: u64,
}

#[derive(Debug)]
pub enum Event {
    Register { src: OwnedFd, dst: OwnedFd, sender: oneshot::Sender<Option<u64>> },
    RemoveRelay { id: u64 },
    GetStatus { sender: oneshot::Sender<Status> },
    ListRelays { sender: oneshot::Sender<Vec<RelayInfo>> },
    Shutdown,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub active_relays: usize,
    pub total_added: u64,
    pub total_removed: u64,
    pub bytes_transferred: u64,
}
