use std::os::unix::io::OwnedFd;

use tokio::sync::oneshot;

use crate::splice_relay::{Destination, RelayInfo, Status};

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
