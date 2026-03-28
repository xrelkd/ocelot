use std::collections::HashMap;

use nix::{
    sys::{
        wait,
        wait::{WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};
use snafu::ResultExt;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::{
    Error, error,
    reaper::{ReapedProcess, event::Event},
};

pub struct Executor {
    registered_processes: HashMap<Pid, oneshot::Sender<ReapedProcess>>,

    register_receiver: mpsc::UnboundedReceiver<(Pid, oneshot::Sender<ReapedProcess>)>,
}

impl Executor {
    pub(crate) fn new(
        register_receiver: mpsc::UnboundedReceiver<(Pid, oneshot::Sender<ReapedProcess>)>,
    ) -> Self {
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
                Some((pid, sender)) = register_receiver.recv() => Event::RegisterProcess { pid, sender },
                _ = signals.recv() => Event::ReapProcess,
            };

            match event {
                Event::Shutdown => break,
                Event::RegisterProcess { pid, sender } => {
                    tracing::info!("Registering child process with PID {pid} for monitoring");
                    let _unused = registered_processes.insert(pid, sender);
                }
                Event::ReapProcess => {
                    for process in reap_processes() {
                        let ReapedProcess { pid, .. } = process;
                        if let Some(sender) = registered_processes.remove(&pid) {
                            let _ = sender.send(process);
                        }
                    }
                }
            }
        }

        let _pids = reap_processes();
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
