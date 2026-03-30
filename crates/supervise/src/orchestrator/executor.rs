use std::{collections::HashMap, time::Duration};

use snafu::ResultExt;
use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::{
    Error, SpliceRelayBuilder, error,
    orchestrator::{OrchestratorConfig, event::Event},
    reaper::Reaper,
    supervisor::{Supervisor, SupervisorConfig, dependency_registry::DependencyRegistry},
};

pub struct Executor {
    supervisors: Vec<SupervisorConfig>,
    shutdown_timeout: Duration,

    event_sender: mpsc::UnboundedSender<Event>,
    event_receiver: mpsc::UnboundedReceiver<Event>,
}

impl Executor {
    pub fn new(
        OrchestratorConfig { supervisors, shutdown_timeout }: OrchestratorConfig,
    ) -> (Self, mpsc::UnboundedSender<Event>) {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        (
            Self {
                supervisors,
                event_sender: event_sender.clone(),
                event_receiver,
                shutdown_timeout,
            },
            event_sender,
        )
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Too many lines due to explicit error handling and task spawning patterns"
    )]
    pub async fn serve(self) -> Result<(), Error> {
        let Self {
            event_sender,
            mut event_receiver,
            supervisors: supervisor_configs,
            shutdown_timeout,
        } = self;

        let (splice_relay, splice_relay_executor) =
            SpliceRelayBuilder::new().build().context(error::BuildSpliceRelaySnafu)?;
        let (reaper, reaper_executor) = Reaper::new();
        let cancel_token = CancellationToken::new();
        let dependency_registry = DependencyRegistry::new(1024);

        let mut supervisor_executors = Vec::with_capacity(supervisor_configs.len());
        let mut supervisors = HashMap::new();
        for supervisor in supervisor_configs {
            let name = supervisor.name().to_string();
            let (supervisor, executor) = Supervisor::new(
                supervisor,
                reaper.clone(),
                splice_relay.clone(),
                dependency_registry.clone(),
            );
            let _unused_supervisor = supervisors.insert(name.clone(), supervisor);
            supervisor_executors.push(executor);
        }

        let mut tasks = JoinSet::<()>::new();

        let _unused = tasks.spawn({
            let cancel_token = cancel_token.clone();
            async move {
                drop(reaper_executor.serve(cancel_token).await);
            }
        });
        let _unused = tasks.spawn({
            let cancel_token = cancel_token.clone();
            async move {
                drop(splice_relay_executor.serve(cancel_token).await);
            }
        });
        let _unused = tasks.spawn({
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .context(error::CreateSignalHandlerSnafu)?;
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .context(error::CreateSignalHandlerSnafu)?;
            let event_sender = event_sender.clone();
            let cancel_token = cancel_token.clone();
            async move {
                tokio::select! {
                    () = cancel_token.cancelled() => {},
                    _ = sigint.recv() => {
                        tracing::info!("Received SIGINT, initiating shutdown");
                        drop(event_sender.send(Event::Shutdown));
                    },
                    _ = sigterm.recv() => {
                        tracing::info!("Received SIGTERM, initiating shutdown");
                        drop(event_sender.send(Event::Shutdown));
                    },
                }
            }
        });

        for executor in supervisor_executors {
            let cancel_token = cancel_token.clone();
            let _unused = tasks.spawn(async move {
                drop(executor.run(cancel_token).await);
            });
        }

        while let Some(event) = event_receiver.recv().await {
            match event {
                Event::Shutdown => break,
                Event::StopSupervisor { name, resp } => {
                    if let Some(supervisor) = supervisors.remove(&name) {
                        supervisor.shutdown();
                        let _ = resp.send(true);
                    } else {
                        let _ = resp.send(false);
                    }
                }
                Event::RestartSupervisor { name, resp } => {
                    if let Some(supervisor) = supervisors.get(&name) {
                        supervisor.start();
                        let _ = resp.send(true);
                    } else {
                        let _ = resp.send(false);
                    }
                }
                Event::GetAllStatuses { resp } => {
                    let mut statuses = HashMap::new();
                    for (name, supervisor) in &supervisors {
                        let status = supervisor.get_status().await;
                        let _unused_status = statuses.insert(name.clone(), status);
                    }
                    drop(resp.send(statuses));
                }
            }

            while tasks.try_join_next().is_some() {}
        }

        tracing::info!("Shutdown all supervisors");
        for supervisor in supervisors.into_values() {
            supervisor.shutdown();
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        cancel_token.cancel();
        let shutdown_result = tokio::time::timeout(shutdown_timeout, async {
            while tasks.join_next().await.is_some() {}
        })
        .await;

        if shutdown_result.is_err() {
            tracing::warn!("Shutdown timeout of {:?} exceeded, forcing exit", shutdown_timeout);
        }

        tracing::info!("Orchestrator shutdown complete");

        Ok(())
    }
}
