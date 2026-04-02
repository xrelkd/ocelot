use nix::{sys::signal::Signal, unistd::Pid};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    Error, SpliceRelay,
    reaper::Reaper,
    splice_relay::Destination,
    supervisor::{
        LogDestination, Phase,
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
    splice_relay: SpliceRelay,
}

impl Executor {
    #[must_use]
    pub const fn new(
        config: Config,
        reaper: Reaper,
        event_sender: mpsc::UnboundedSender<Event>,
        event_receiver: mpsc::UnboundedReceiver<Event>,
        dependency_registry: DependencyRegistry,
        splice_relay: SpliceRelay,
    ) -> Self {
        Self { reaper, config, event_sender, event_receiver, dependency_registry, splice_relay }
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
        let Self {
            config,
            reaper,
            event_sender,
            mut event_receiver,
            dependency_registry,
            splice_relay,
        } = self;
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
                    if let Some(pgid) = state.process_group_id() {
                        let signal = config.shutdown_signal.unwrap_or(Signal::SIGTERM);
                        forward_signal_group(pgid, signal);

                        // Spawn a grace-period timer that escalates to SIGKILL
                        // if the process has not been reaped in time.
                        let grace_period = config.termination_grace_period;
                        tasks.schedule_sigkill_timeout(cancel_token.clone(), pgid, grace_period);
                    }
                    state.set_shutting_down(config.termination_grace_period);
                    break;
                }
                (Event::Shutdown, Phase::ShuttingDown) => {
                    if let Some(pgid) = state.process_group_id() {
                        forward_signal_group(pgid, Signal::SIGKILL);
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
                (Event::LogReady, Phase::Running) => {
                    state.notify_log_ready();
                }
                (Event::Start, Phase::Pending | Phase::CrashLoopBackOff | Phase::Failed { .. }) => {
                    state.set_starting();
                    match config.command().spawn().await {
                        Ok(spawned_process) => {
                            ProcessSpawnContext {
                                config: &config,
                                state: &mut state,
                                cancel_token: cancel_token.clone(),
                                event_sender: &event_sender,
                                splice_relay: splice_relay.clone(),
                                reaper: &reaper,
                                tasks: &mut tasks,
                                spawned_process,
                            }
                            .spawn();
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
                    if should_kill && let Some(pgid) = state.process_group_id() {
                        // Send `SIGKILL` to kill the process and the process will be restarted
                        // while handling `Event::ProcessReaped`.
                        forward_signal_group(pgid, Signal::SIGKILL);
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
                (Event::GetStatus { resp }, _) => {
                    let _ = resp.send(state.to_status());
                }
                _ => {}
            }

            while tasks.try_join_next().is_some() {}
        }

        // Wait for all remaining tasks to complete before exit
        while tasks.join_next().await.is_some() {}

        Ok(())
    }
}

struct ProcessSpawnContext<'a> {
    config: &'a Config,
    state: &'a mut State,
    cancel_token: CancellationToken,
    event_sender: &'a mpsc::UnboundedSender<Event>,
    splice_relay: SpliceRelay,
    reaper: &'a Reaper,
    tasks: &'a mut tokio::task::JoinSet<()>,
    spawned_process: SpawnedProcess,
}

impl ProcessSpawnContext<'_> {
    fn spawn(self) {
        let ProcessSpawnContext {
            config,
            state,
            cancel_token,
            event_sender,
            splice_relay,
            reaper,
            tasks,
            spawned_process: SpawnedProcess { pid, pgid, stdout_fd, stderr_fd },
        } = self;

        tracing::info!("Started process `{}` with PID `{pid}` PGID `{pgid}`", config.name());
        state.set_running(pid, pgid);

        // Handle stdout logging based on config
        if let Some(stdout_fd) = stdout_fd {
            match &config.log_stdout.destination {
                LogDestination::Null => {
                    // Should not happen: stdout_fd exists but destination Null means it should be
                    // discarded.
                    tracing::warn!("stdout_fd present but log destination is Null; ignoring");
                }
                LogDestination::Inherit => {
                    tasks.register_splice_relay(
                        cancel_token.clone(),
                        event_sender,
                        splice_relay.clone(),
                        stdout_fd,
                        Destination::Stdout,
                    );
                }
                LogDestination::File { path } => {
                    tasks.register_file_logging(
                        cancel_token.clone(),
                        event_sender,
                        stdout_fd,
                        path,
                        config.log_stdout.rotation.clone().unwrap_or_default(),
                    );
                }
            }
        }

        // Handle stderr logging based on config
        if let Some(stderr_fd) = stderr_fd {
            match &config.log_stderr.destination {
                LogDestination::Null => {
                    // Should not happen: stderr_fd exists but destination Null means it should be
                    // discarded.
                    tracing::warn!("stderr_fd present but log destination is Null; ignoring");
                }
                LogDestination::Inherit => {
                    tasks.register_splice_relay(
                        cancel_token.clone(),
                        event_sender,
                        splice_relay,
                        stderr_fd,
                        Destination::Stderr,
                    );
                }
                LogDestination::File { path } => {
                    tasks.register_file_logging(
                        cancel_token.clone(),
                        event_sender,
                        stderr_fd,
                        path,
                        config.log_stderr.rotation.clone().unwrap_or_default(),
                    );
                }
            }
        }

        tasks.wait_for_reap(
            cancel_token,
            event_sender,
            reaper,
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
}

/// Sends a signal to an entire process group.
///
/// Using a negative PID signals all processes in the group (POSIX semantics).
/// This ensures that child processes spawned by the supervised process
/// (e.g., sshd sessions) also receive the signal.
fn forward_signal_group(pgid: Pid, signal: Signal) {
    tracing::info!("Sending signal {signal:?} to process group {pgid}");
    // kill(-pgid, signal) targets the entire process group
    let group_pid = Pid::from_raw(-pgid.as_raw());
    if let Err(err) = nix::sys::signal::kill(group_pid, signal) {
        tracing::warn!("Failed to send signal to process group: {err}");
    }
}
