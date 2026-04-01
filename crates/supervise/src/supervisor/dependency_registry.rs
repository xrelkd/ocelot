use std::collections::HashMap;

use snafu::ResultExt;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::{
    Error, error,
    supervisor_config::{DependencyCondition, ProcessDependency},
};

#[derive(Clone)]
pub struct DependencyRegistry {
    tx: broadcast::Sender<DependencyEvent>,
}

impl DependencyRegistry {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn create_notifier(&self, name: impl Into<String>) -> DependencyNotifier {
        DependencyNotifier { name: name.into(), tx: self.tx.clone() }
    }

    #[must_use]
    pub fn create_waiter(&self, deps: HashMap<String, ProcessDependency>) -> DependencyWaiter {
        DependencyWaiter { rx: self.tx.subscribe(), pending: deps }
    }
}

#[derive(Clone)]
pub struct DependencyNotifier {
    name: String,
    tx: broadcast::Sender<DependencyEvent>,
}

impl DependencyNotifier {
    pub fn notify_started(&self) {
        drop(self.tx.send(DependencyEvent::Started { name: self.name.clone() }));
    }

    pub fn notify_healthy(&self) {
        drop(self.tx.send(DependencyEvent::Healthy { name: self.name.clone() }));
    }

    pub fn notify_completed(&self, exit_code: i32) {
        drop(self.tx.send(DependencyEvent::Completed { name: self.name.clone(), exit_code }));
    }

    pub fn notify_log_ready(&self) {
        drop(self.tx.send(DependencyEvent::LogReady { name: self.name.clone() }));
    }
}

pub struct DependencyWaiter {
    rx: broadcast::Receiver<DependencyEvent>,
    pending: HashMap<String, ProcessDependency>,
}

impl DependencyWaiter {
    pub async fn wait(mut self, cancel_token: &CancellationToken) -> Result<(), Error> {
        while !self.pending.is_empty() {
            let event = tokio::select! {
                () = cancel_token.cancelled() => return Ok(()),
                res = self.rx.recv() => res.context(error::ReceiveDependencySnafu)?,
            };
            let should_break = self.handle_event(event);
            if should_break {
                break;
            }
        }
        Ok(())
    }

    #[inline]
    fn handle_event(&mut self, event: DependencyEvent) -> bool {
        match event {
            DependencyEvent::Started { name } => {
                self.check_satisfied(&name, DependencyCondition::Started);
            }
            DependencyEvent::Healthy { name } => {
                self.check_satisfied(&name, DependencyCondition::Healthy);
            }
            DependencyEvent::Completed { name, exit_code } => {
                let cond = if self.pending.contains_key(&name) && exit_code == 0 {
                    DependencyCondition::CompletedSuccessfully
                } else {
                    DependencyCondition::Completed
                };
                self.check_satisfied(&name, cond);
            }
            DependencyEvent::LogReady { name } => {
                self.check_satisfied(&name, DependencyCondition::LogReady);
            }
        }
        self.pending.is_empty()
    }

    fn check_satisfied(&mut self, name: &str, cond: DependencyCondition) {
        if let Some(required) = self.pending.get(name)
            && (required.condition == Some(cond) || required.condition.is_none())
        {
            let _unused = self.pending.remove(name);
            tracing::debug!(dependency = name, condition = ?cond, "Dependency satisfied");
        }
    }
}

#[derive(Clone, Debug)]
enum DependencyEvent {
    Started { name: String },
    Healthy { name: String },
    Completed { name: String, exit_code: i32 },
    LogReady { name: String },
}
