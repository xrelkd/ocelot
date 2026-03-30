use std::{
    collections::HashMap,
    os::unix::io::OwnedFd,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};

use nix::{
    fcntl,
    fcntl::SpliceFFlags,
    poll::PollTimeout,
    sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
    unistd,
};
use snafu::ResultExt;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::splice_relay::{
    Destination, Event, RelayEntry, RelayInfo, Status, Waker,
    config::Config,
    error::{self, Error},
};

const EVENT_FD_TOKEN: u64 = u64::MAX;

pub struct Executor {
    config: Config,
    event_sender: mpsc::Sender<Event>,
    event_receiver: mpsc::Receiver<Event>,
    waker: Waker,
}

impl Executor {
    pub const fn new(
        config: Config,
        event_sender: mpsc::Sender<Event>,
        event_receiver: mpsc::Receiver<Event>,
        waker: Waker,
    ) -> Self {
        Self { config, event_sender, event_receiver, waker }
    }

    pub async fn serve(self, cancel_token: CancellationToken) -> Result<(), Error> {
        let Self { config, event_sender, event_receiver, waker } = self;

        let join_handle = std::thread::spawn({
            let waker = waker.clone();
            move || match ThreadWorker::new(config, event_receiver, waker) {
                Ok(mut worker) => worker.run(),
                Err(e) => {
                    tracing::error!("Failed to create worker: {:?}", e);
                }
            }
        });

        cancel_token.cancelled().await;
        drop(event_sender.send(Event::Shutdown));
        waker.wake();
        let _unused = join_handle.join();
        tracing::debug!("Worker thread shut down cleanly");
        Ok(())
    }
}

struct ThreadWorker {
    config: Config,
    event_sender: mpsc::Receiver<Event>,
    waker: Waker,
    relays: HashMap<u64, RelayEntry>,
    epoll: Epoll,
    next_id: AtomicU64,
    status: Status,
}

impl ThreadWorker {
    fn new(
        config: Config,
        event_sender: mpsc::Receiver<Event>,
        waker: Waker,
    ) -> Result<Self, Error> {
        let epoll = Epoll::new(EpollCreateFlags::empty()).context(error::CreateEpollSnafu)?;
        {
            let eventfd_event = EpollEvent::new(EpollFlags::EPOLLIN, EVENT_FD_TOKEN);
            epoll.add(waker.as_ref(), eventfd_event).context(error::AddEpollFdSnafu)?;
        }

        Ok(Self {
            config,
            event_sender,
            waker,
            relays: HashMap::new(),
            epoll,
            next_id: AtomicU64::new(1),
            status: Status::default(),
        })
    }

    fn run(&mut self) {
        let mut events = [EpollEvent::empty(); 256];

        loop {
            // Use `NONE` because the waker fd is registered with epoll.
            // The waker will wake epoll when there's new work (register, remove, shutdown),
            // so periodic wakeups are unnecessary. This is crucial for lightweight PID 1.
            let timeout = PollTimeout::NONE;

            let num_events = match self.epoll.wait(&mut events, timeout) {
                Ok(n) => n,
                Err(e) => {
                    tracing::warn!("Epoll wait error: {:?}", e);
                    continue;
                }
            };

            for event in events.iter().take(num_events) {
                let token = event.data();

                if token == EVENT_FD_TOKEN {
                    let should_shutdown = self.handle_control_events();
                    if should_shutdown {
                        return;
                    }
                } else {
                    self.handle_io_event(token);
                }
            }
        }
    }

    fn handle_control_events(&mut self) -> bool {
        let mut buf = [0u8; 8];
        let _ = unistd::read(self.waker.as_ref(), &mut buf);

        while let Ok(event) = self.event_sender.try_recv() {
            match event {
                Event::Register { source: src, destination, sender, start_notification } => {
                    let result = self.register(src, destination, start_notification);
                    let _unused = sender.send(result.ok());
                }
                Event::RemoveRelay { id } => {
                    self.remove(id);
                }
                Event::GetStatus { sender } => {
                    let _unused = sender.send(self.get_status());
                }
                Event::ListRelays { sender } => {
                    let _unused = sender.send(self.list_relays());
                }
                Event::Shutdown => {
                    return true;
                }
            }
        }
        false
    }

    fn handle_io_event(&mut self, token: u64) {
        let flags = SpliceFFlags::SPLICE_F_MOVE | SpliceFFlags::SPLICE_F_NONBLOCK;
        // Determine if we need to remove this token, and possibly send notification.
        let remove_id = {
            if let Some(entry) = self.relays.get_mut(&token) {
                let dst_fd: &dyn std::os::unix::io::AsFd = match &entry.destination {
                    Destination::Stdout => &std::io::stdout(),
                    Destination::Stderr => &std::io::stderr(),
                    Destination::OwnedFd { fd } => fd,
                };
                let result =
                    fcntl::splice(&entry.source, None, dst_fd, None, self.config.chunk_size, flags);
                match result {
                    Ok(0) => Some(entry.id),
                    Ok(n) => {
                        self.status.bytes_transferred += n as u64;
                        if let Some(notify_sender) = entry.start_notification.take() {
                            let _ = notify_sender.send(());
                        }
                        None
                    }
                    Err(nix::errno::Errno::EAGAIN) => None,
                    Err(_) => Some(entry.id),
                }
            } else {
                None
            }
        };
        if let Some(id) = remove_id {
            self.remove(id);
        }
    }

    fn register(
        &mut self,
        source: OwnedFd,
        destination: Destination,
        start_notification: Option<oneshot::Sender<()>>,
    ) -> Result<u64, Error> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let epoll_ev = EpollEvent::new(EpollFlags::EPOLLIN, id);
        self.epoll.add(&source, epoll_ev).context(error::AddEpollFdSnafu)?;

        let entry = RelayEntry::new(id, source, destination, start_notification);
        let _unused = self.relays.insert(id, entry);
        self.status.active_relays = self.relays.len();
        self.status.total_added += 1;

        Ok(id)
    }

    fn remove(&mut self, id: u64) {
        if let Some(entry) = self.relays.remove(&id) {
            let _ = self.epoll.delete(&entry.source);
            self.status.active_relays = self.relays.len();
            self.status.total_removed += 1;
        }
    }

    fn get_status(&self) -> Status { self.status.clone() }

    fn list_relays(&self) -> Vec<RelayInfo> {
        self.relays.values().map(|e| RelayInfo { id: e.id }).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::io::OwnedFd;

    use nix::{fcntl::OFlag, unistd::pipe2};

    use crate::splice_relay::{Destination, RelayEntry, RelayInfo, Status, config::Config};

    fn create_pipe() -> (OwnedFd, OwnedFd) {
        let (r, w) = pipe2(OFlag::O_NONBLOCK).expect("pipe2 failed");
        (r, w)
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.buffer_size, 128 * 1024);
        assert_eq!(config.chunk_size, 128 * 1024);
    }

    #[test]
    fn test_config_new() {
        let config = Config::new(1024, 2048);
        assert_eq!(config.buffer_size, 1024);
        assert_eq!(config.chunk_size, 2048);
    }

    #[test]
    fn test_config_with_buffer_size() {
        let config = Config::default().with_buffer_size(4096);
        assert_eq!(config.buffer_size, 4096);
    }

    #[test]
    fn test_config_with_chunk_size() {
        let config = Config::default().with_chunk_size(4096);
        assert_eq!(config.chunk_size, 4096);
    }

    #[test]
    fn test_status_default() {
        let status = Status::default();
        assert_eq!(status.active_relays, 0);
        assert_eq!(status.total_added, 0);
        assert_eq!(status.total_removed, 0);
        assert_eq!(status.bytes_transferred, 0);
    }

    #[test]
    fn test_relay_entry_new() {
        let (src, dst) = create_pipe();
        let entry = RelayEntry::new(42, src, Destination::OwnedFd { fd: dst }, None);
        assert_eq!(entry.id, 42);
    }

    #[test]
    fn test_relay_info_new() {
        let info = RelayInfo { id: 42 };
        assert_eq!(info.id, 42);
    }
}
