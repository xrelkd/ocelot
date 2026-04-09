use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use nix::{
    sys::{
        wait,
        wait::{WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};
use snafu::ResultExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    Error, error,
    reaper::{ReapedProcess, RegisteredProcess, event::Event},
};

pub struct Executor {
    registered_processes: HashMap<Pid, RegisteredProcess>,

    register_receiver: mpsc::UnboundedReceiver<RegisteredProcess>,
}

impl Executor {
    pub(crate) fn new(register_receiver: mpsc::UnboundedReceiver<RegisteredProcess>) -> Self {
        Self { register_receiver, registered_processes: HashMap::new() }
    }

    /// Serves as the executor for the reaper, handling child process reap
    /// events.
    ///
    /// # Errors
    ///
    /// Returns an error if unable to create the signal handler for SIGCHLD.
    pub async fn serve(self, cancel_token: CancellationToken) -> Result<(), Error> {
        let Self { mut registered_processes, mut register_receiver } = self;

        let mut signals = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())
            .context(error::CreateSignalHandlerSnafu)?;

        loop {
            let event = tokio::select! {
                () = cancel_token.cancelled() => Event::Shutdown,
                Some(registered_process) = register_receiver.recv() => Event::RegisterProcess { registered_process },
                _ = signals.recv() => Event::ReapProcess,
            };

            match event {
                Event::Shutdown => break,
                Event::RegisterProcess { registered_process } => {
                    let pid = registered_process.pid;
                    tracing::info!("Registering child process with PID {pid} for monitoring");
                    let _unused = registered_processes.insert(pid, registered_process);
                }
                Event::ReapProcess => {
                    for process in reap_processes() {
                        let ReapedProcess { pid, .. } = process;
                        if let Some(registered) = registered_processes.remove(&pid) {
                            let _ = registered.sender.send(process);
                        }
                    }
                }
            }
        }

        // Shutdown phase: actively kill processes that exceed their individual
        // grace periods, rather than passively waiting for the maximum.
        if !registered_processes.is_empty() {
            // Map each PID to its kill deadline (registration_time + grace_period).
            let mut deadlines = {
                let now = Instant::now();
                registered_processes
                    .iter()
                    .map(|(&pid, process)| (pid, now + process.termination_grace_period))
                    .collect::<HashMap<Pid, Instant>>()
            };

            loop {
                // Find the earliest deadline among still-active processes.
                let earliest = deadlines.values().min().copied();

                let Some(deadline) = earliest else {
                    // No more processes to wait for.
                    break;
                };

                // Compute the interval to sleep, with a reasonable minimum.
                let sleep_duration = deadline
                    .checked_duration_since(Instant::now())
                    .unwrap_or(Duration::from_millis(10));

                tokio::select! {
                    () = tokio::time::sleep(sleep_duration) => {},
                    _ = signals.recv() => {}
                }

                // Reap any processes that have already exited.
                for process @ ReapedProcess { pid, .. } in reap_processes() {
                    if let Some(registered) = registered_processes.remove(&pid) {
                        let _unused = deadlines.remove(&pid);
                        let _ = registered.sender.send(process);
                    }
                }

                if registered_processes.is_empty() {
                    break;
                }

                // Kill processes whose deadlines have elapsed.
                let overdue = {
                    let now = Instant::now();
                    deadlines
                        .iter()
                        .filter_map(
                            |(&pid, &deadline)| if now >= deadline { Some(pid) } else { None },
                        )
                        .collect::<Vec<Pid>>()
                };

                for pid in overdue {
                    tracing::warn!("Grace period exceeded for process {pid}, sending SIGKILL");
                    if let Err(err) = nix::sys::signal::kill(pid, nix::sys::signal::Signal::SIGKILL)
                    {
                        tracing::warn!("Failed to SIGKILL process {pid}: {err}");
                    }
                    // Remove from tracking; we won't wait further for this process.
                    let _unused = registered_processes.remove(&pid);
                    let _unused = deadlines.remove(&pid);
                }

                if registered_processes.is_empty() {
                    break;
                }
            }
        }

        let pid_count = reap_processes().len();
        if pid_count > 0 {
            tracing::info!("Reaped {pid_count} process(es)");
        }

        Ok(())
    }
}

fn reap_processes() -> Vec<ReapedProcess> {
    tracing::info!("Reaping any remaining zombie child processes...");
    std::iter::from_fn(|| {
        let Ok(status) = wait::waitpid(None, Some(WaitPidFlag::WNOHANG)) else {
            return None;
        };
        match status {
            WaitStatus::Exited(pid, exit_code) => {
                tracing::info!("Reaped child process {pid} with exit code {exit_code}");
                Some(ReapedProcess { pid, exit_code })
            }
            WaitStatus::Signaled(pid, sig, _) => {
                let exit_code = 128 + sig as i32;
                tracing::info!(
                    "Reaped child process {pid} terminated by signal {sig} (exit code {exit_code})"
                );
                Some(ReapedProcess { pid, exit_code })
            }
            _ => None,
        }
    })
    .collect()
}
