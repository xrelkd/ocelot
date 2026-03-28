use std::{collections::HashMap, time::Duration};

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

const DEFAULT_TERMINATION_GRACE_PERIOD: Duration = Duration::from_millis(200);

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
                        if let Some(RegisteredProcess { sender, .. }) =
                            registered_processes.remove(&pid)
                        {
                            let _ = sender.send(process);
                        }
                    }
                }
            }
        }

        if !registered_processes.is_empty() {
            let deadline = {
                let timeout = registered_processes
                    .values()
                    .map(|p| p.termination_grace_period)
                    .max()
                    .unwrap_or(DEFAULT_TERMINATION_GRACE_PERIOD);
                tokio::time::Instant::now() + timeout
            };
            loop {
                tokio::select! {
                    () = tokio::time::sleep_until(deadline) => break,
                    _ = signals.recv() => {}
                }

                for process in reap_processes() {
                    let ReapedProcess { pid, .. } = process;
                    if let Some(RegisteredProcess { sender, .. }) =
                        registered_processes.remove(&pid)
                    {
                        let _ = sender.send(process);
                    }
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
                tracing::info!("Reaped child process {pid} terminated by signal {sig}");
                None
            }
            _ => None,
        }
    })
    .collect()
}
