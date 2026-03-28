use nix::sys::signal::Signal;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    Error,
    reaper::Reaper,
    supervisor::{
        Phase, ProcessStatus,
        config::Config,
        dependency_registry::DependencyRegistry,
        event::Event,
        spawned_process::{CommandExt, SpawnedProcess},
        state::State,
        task_runner::TaskRunner,
    },
};

pub struct Executor {
    reaper: Reaper,
    config: Config,
    event_sender: mpsc::UnboundedSender<Event>,
    event_receiver: mpsc::UnboundedReceiver<Event>,
    dependency_registry: DependencyRegistry,
}

impl Executor {
    #[must_use]
    pub const fn new(
        config: Config,
        reaper: Reaper,
        event_sender: mpsc::UnboundedSender<Event>,
        event_receiver: mpsc::UnboundedReceiver<Event>,
        dependency_registry: DependencyRegistry,
    ) -> Self {
        Self { reaper, config, event_sender, event_receiver, dependency_registry }
    }

    /// Gracefully shuts down supervised processes by sending SIGTERM first,
    /// then SIGKILL after `termination_grace_period` if the process doesn't
    /// exit.
    ///
    /// # Errors
    ///
    /// Returns an error if signal handlers cannot be created or if there are
    /// issues with task spawning.
    #[expect(
        clippy::too_many_lines,
        reason = "This is the main event loop containing the core state machine; refactoring \
                  would require extracting the match arms into separate functions which would \
                  reduce readability and increase complexity"
    )]
    pub async fn run(self, cancel_token: CancellationToken) -> Result<(), Error> {
        let Self { config, reaper, event_sender, mut event_receiver, dependency_registry } = self;
        let dependency_waiter = dependency_registry.create_waiter(config.depends_on.clone());
        let dependency_notifier = dependency_registry.create_notifier(&config.name);
        let mut state = State::new(dependency_notifier);
        let mut tasks = tokio::task::JoinSet::new();

        // Wait for dependencies if any.
        dependency_waiter.wait(&cancel_token).await?;
        drop(event_sender.send(Event::Start));

        loop {
            let event = tokio::select! {
                () = cancel_token.cancelled() => Event::Shutdown,
                Some(event) = event_receiver.recv() => event,
            };

            match (event, state.phase()) {
                (Event::Shutdown, Phase::Running) => {
                    if let Some(&SpawnedProcess { pid }) = state.spawned() {
                        let signal = config.shutdown_signal.unwrap_or(Signal::SIGTERM);
                        forward_signal(pid, signal);
                        state.set_shutting_down(config.termination_grace_period);
                    }
                    break;
                }
                (Event::Shutdown, Phase::ShuttingDown) => {
                    if let Some(&SpawnedProcess { pid }) = state.spawned() {
                        forward_signal(pid, Signal::SIGKILL);
                    }
                    break;
                }
                (Event::Shutdown, _) => break,
                (Event::ProcessReaped { .. }, Phase::ShuttingDown) => {
                    state.clear_shutdown_deadline();
                    break;
                }
                (Event::ProcessReaped { exit_code }, _) => {
                    state.set_exited(exit_code);
                    if let Some(interval) = state.next_interval(&config.restart_policy) {
                        tasks.schedule(cancel_token.clone(), &event_sender, interval, Event::Start);
                    }
                }
                (Event::Start, Phase::Pending | Phase::CrashLoopBackOff | Phase::Failed { .. }) => {
                    state.set_starting();
                    match config.command().spawn().await {
                        Ok(spawned @ SpawnedProcess { pid }) => {
                            tracing::info!("Started process `{}` with PID `{pid}`", config.name());
                            state.set_running(spawned);
                            tasks.wait_for_reap(
                                cancel_token.clone(),
                                &event_sender,
                                &reaper,
                                pid,
                                config.termination_grace_period,
                            );
                            if config.liveness_probe.is_some() {
                                drop(event_sender.send(Event::CheckLiveness));
                            }
                            if config.readiness_probe.is_some() {
                                drop(event_sender.send(Event::CheckReadiness));
                            }
                        }
                        Err(err) => {
                            tracing::error!("Failed to start process: {err}");
                            state.set_failed(-1);
                        }
                    }
                }
                (Event::CheckReadiness, Phase::Running) => {
                    if let Some(probe) = &config.readiness_probe {
                        tasks.check_readiness(cancel_token.clone(), &event_sender, probe.clone());
                    }
                }
                (Event::ReadinessChecked { ready }, _) => {
                    state.set_ready(ready);
                    if let Some(probe) = &config.readiness_probe {
                        tasks.schedule(
                            cancel_token.clone(),
                            &event_sender,
                            probe.period,
                            Event::CheckReadiness,
                        );
                    }
                }
                (Event::CheckLiveness, Phase::Running) => {
                    if let Some(probe) = &config.liveness_probe {
                        tasks.check_liveness(cancel_token.clone(), &event_sender, probe.clone());
                    }
                }
                (Event::LivenessChecked { should_kill }, _) => {
                    if should_kill && let Some(&SpawnedProcess { pid }) = state.spawned() {
                        // Send `SIGKILL` to kill the process and the process will be restarted
                        // while handling `Event::ProcessReaped`.
                        forward_signal(pid, Signal::SIGKILL);
                        state.set_failed(-1);
                    } else if let Some(probe) = &config.liveness_probe {
                        tasks.schedule(
                            cancel_token.clone(),
                            &event_sender,
                            probe.period,
                            Event::CheckLiveness,
                        );
                    }
                }
                (Event::ForwardSignal { signal }, Phase::Running) => {
                    if let Some(&SpawnedProcess { pid }) = state.spawned() {
                        forward_signal(pid, signal);
                    }
                }
                (Event::GetStatus { resp }, _) => {
                    let status = ProcessStatus {
                        phase: state.phase(),
                        restart_count: state.restart_count(),
                        last_exit_code: state.last_exit_code(),
                        ready: state.ready(),
                    };
                    let _ = resp.send(status);
                }
                _ => {}
            }

            if matches!(state.phase(), Phase::ShuttingDown) && state.shutdown_deadline_exceeded() {
                if let Some(&SpawnedProcess { pid }) = state.spawned() {
                    forward_signal(pid, Signal::SIGKILL);
                    tracing::warn!("Grace period exceeded, sending SIGKILL to process {pid}");
                }
                break;
            }

            while tasks.try_join_next().is_some() {}
        }

        while tasks.join_next().await.is_some() {}

        Ok(())
    }
}

fn forward_signal(pid: nix::unistd::Pid, signal: Signal) {
    tracing::info!("Sending signal {signal:?} to process {pid}");
    if let Err(e) = nix::sys::signal::kill(pid, signal) {
        tracing::warn!("Failed to send signal to process: {}", e);
    }
}
