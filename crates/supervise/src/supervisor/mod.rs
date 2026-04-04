pub mod config;
pub mod dependency_registry;
mod event;
mod executor;
pub mod probe;
mod spawned_process;
mod state;
mod task_runner;

use tokio::sync::{mpsc, oneshot};

use self::event::Event;
pub use self::{
    config::{Config as SupervisorConfig, LogDestination, LogStreamConfig, RestartPolicy},
    dependency_registry::DependencyRegistry,
    executor::Executor as SupervisorExecutor,
};
use crate::{Reaper, SpliceRelay};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Pending,
    Running,
    ShuttingDown,
    CrashLoopBackOff,
    Completed,
    Failed { exit_code: i32 },
}

impl Phase {
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. } | Self::CrashLoopBackOff)
    }
}

#[derive(Clone, Debug)]
pub struct ProcessStatus {
    pub phase: Phase,
    pub restart_count: u32,
    pub last_exit_code: Option<i32>,
    pub ready: bool,
}

#[derive(Clone)]
pub struct Supervisor {
    event_sender: mpsc::UnboundedSender<Event>,
}

impl Supervisor {
    #[must_use]
    pub fn new(
        config: SupervisorConfig,
        reaper: Reaper,
        splice_relay: SpliceRelay,
        dependency_registry: DependencyRegistry,
    ) -> (Self, SupervisorExecutor) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let handle = Self { event_sender: event_sender.clone() };
        let executor = SupervisorExecutor::new(
            config,
            reaper,
            event_sender,
            event_receiver,
            dependency_registry,
            splice_relay,
        );
        (handle, executor)
    }

    #[tracing::instrument(name = "Supervisor::start", skip_all)]
    pub fn start(&self) { drop(self.event_sender.send(Event::Start)); }

    #[tracing::instrument(name = "Supervisor::shutdown", skip_all)]
    pub fn shutdown(self) { drop(self.event_sender.send(Event::Shutdown)); }

    #[tracing::instrument(name = "Supervisor::get_status", skip_all)]
    pub async fn get_status(&self) -> ProcessStatus {
        let (sender, receiver) = oneshot::channel::<ProcessStatus>();
        drop(self.event_sender.send(Event::GetStatus { resp: sender }));
        receiver.await.unwrap_or(ProcessStatus {
            phase: Phase::Pending,
            restart_count: 0,
            last_exit_code: None,
            ready: false,
        })
    }
}
