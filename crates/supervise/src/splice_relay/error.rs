use std::sync::mpsc;

use snafu::Snafu;
use tokio::sync::oneshot;

use crate::splice_relay::event::Event;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create eventfd: {source}"))]
    CreateEventfd { source: nix::Error },

    #[snafu(display("Failed to create epoll instance: {source}"))]
    CreateEpoll { source: nix::Error },

    #[snafu(display("Failed to add file descriptor to epoll: {source}"))]
    AddEpollFd { source: nix::Error },

    #[snafu(display("Failed to delete file descriptor from epoll: {source}"))]
    DelEpollFd { source: nix::Error },

    #[snafu(display("Failed to splice data: {source}"))]
    SpliceData { source: nix::Error },

    #[snafu(display("Failed to write to eventfd: {source}"))]
    WriteEventfd { source: nix::Error },

    #[snafu(display("Failed to read from eventfd: {source}"))]
    ReadEventfd { source: nix::Error },

    #[snafu(display("Failed to send event: {source}"))]
    SendEvent { source: mpsc::SendError<Event> },

    #[snafu(display("Failed to receive event: {source}"))]
    RecvEvent { source: oneshot::error::RecvError },

    #[snafu(display("Relay not found: {id}"))]
    RelayNotFound { id: u64 },

    #[snafu(display("Failed to clone file descriptor: {source}"))]
    CloneFd { source: nix::Error },
}
