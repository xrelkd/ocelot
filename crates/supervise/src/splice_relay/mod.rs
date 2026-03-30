mod config;
mod destination;
mod error;
mod event;
mod executor;
mod relay_entry;
mod relay_info;
mod status;
mod waker;

#[cfg(test)]
mod tests;

use std::{os::unix::io::OwnedFd, sync::mpsc};

use tokio::sync::oneshot;

pub use self::{
    config::Config, destination::Destination, error::Error, event::Event, relay_entry::RelayEntry,
    relay_info::RelayInfo, status::Status,
};
use self::{executor::Executor, waker::Waker};

pub struct Builder {
    config: Config,
}

impl Builder {
    #[must_use]
    pub fn new() -> Self { Self { config: Config::default() } }

    #[must_use]
    pub const fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    pub const fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.config.buffer_size = buffer_size;
        self
    }

    #[must_use]
    pub const fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.config.chunk_size = chunk_size;
        self
    }

    #[expect(
        clippy::missing_errors_doc,
        reason = "Error type is crate::Error and is already documented in error module"
    )]
    pub fn build(self) -> Result<(SpliceRelay, Executor), Error> {
        let waker = Waker::new()?;
        let (event_sender, event_receiver) = mpsc::channel();
        let executor =
            Executor::new(self.config, event_sender.clone(), event_receiver, waker.clone());
        Ok((SpliceRelay { event_sender, waker }, executor))
    }
}

impl Default for Builder {
    fn default() -> Self { Self::new() }
}

#[derive(Clone, Debug)]
pub struct SpliceRelay {
    event_sender: mpsc::Sender<Event>,
    waker: Waker,
}

#[derive(Debug)]
pub struct RelayRegistration {
    pub id: u64,
    pub started: oneshot::Receiver<()>,
}

impl SpliceRelay {
    pub async fn register(
        &self,
        src: OwnedFd,
        destination: Destination,
    ) -> Option<RelayRegistration> {
        let (id_sender, id_receiver) = oneshot::channel();
        let (notify_sender, notify_receiver) = oneshot::channel();
        if let Err(err) = self.event_sender.send(Event::Register {
            source: src,
            destination,
            sender: id_sender,
            start_notification: Some(notify_sender),
        }) {
            tracing::warn!("{err}");
            None
        } else {
            self.waker.wake();
            match id_receiver.await {
                Ok(Some(id)) => Some(RelayRegistration { id, started: notify_receiver }),
                _ => None,
            }
        }
    }

    #[tracing::instrument(name = "SpliceRelay::remove", skip_all)]
    pub fn remove(&self, id: u64) {
        if self.event_sender.send(Event::RemoveRelay { id }).is_ok() {
            self.waker.wake();
        }
    }

    #[tracing::instrument(name = "SpliceRelay::get_status", skip_all)]
    pub async fn get_status(&self) -> Option<Status> {
        let (sender, receiver) = oneshot::channel();
        if let Err(err) = self.event_sender.send(Event::GetStatus { sender }) {
            tracing::error!("{err}");
            None
        } else {
            self.waker.wake();
            receiver.await.ok()
        }
    }

    #[tracing::instrument(name = "SpliceRelay::list", skip_all)]
    pub async fn list(&self) -> Vec<RelayInfo> {
        let (sender, receiver) = oneshot::channel();
        if self.event_sender.send(Event::ListRelays { sender }).is_ok() {
            self.waker.wake();
            receiver.await.unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}
