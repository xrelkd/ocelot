use std::{os::fd::OwnedFd, path::Path, time::Duration};

use tokio::{
    io::{self, AsyncWriteExt, unix::AsyncFd},
    sync::mpsc,
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;

use crate::{
    Reaper,
    reaper::ReapedProcess,
    rotating_file::RotatingFile,
    splice_relay::{Destination, RelayRegistration, SpliceRelay},
    supervisor::{LogRotationConfig, event::Event, probe::Probe},
};

pub trait TaskRunner {
    fn wait_for_reap(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        reaper: &Reaper,
        pid: nix::unistd::Pid,
        termination_grace_period: Duration,
    );

    fn register_splice_relay(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        relay: SpliceRelay,
        source_fd: OwnedFd,
        destination: Destination,
    );

    fn register_file_logging(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        source_fd: OwnedFd,
        file_path: impl AsRef<Path> + Send,
        rotation: LogRotationConfig,
    );

    fn check_readiness(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        probe: Probe,
    );

    fn check_liveness(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        probe: Probe,
    );

    fn schedule(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        timeout: Duration,
        event: Event,
    );
}

impl TaskRunner for JoinSet<()> {
    fn wait_for_reap(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        reaper: &Reaper,
        pid: nix::unistd::Pid,
        termination_grace_period: Duration,
    ) {
        let receiver = reaper.register(pid, termination_grace_period);
        let event_sender = event_sender.clone();
        let _unused = self.spawn(async move {
            let maybe_reaped_process = tokio::select! {
                rp = receiver => rp,
                () = cancel_token.cancelled() => return,
            };
            let exit_code = if let Ok(ReapedProcess { pid, exit_code }) = maybe_reaped_process {
                tracing::info!("Process {pid} exited with code {exit_code}");
                exit_code
            } else {
                tracing::warn!("Failed to receive exit notification");
                -1
            };
            drop(event_sender.send(Event::ProcessReaped { exit_code }));
        });
    }

    fn register_splice_relay(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        relay: SpliceRelay,
        source_fd: OwnedFd,
        destination: Destination,
    ) {
        let event_sender = event_sender.clone();
        let fut = async move {
            let res = relay.register(source_fd, destination).await;
            if let Some(RelayRegistration { started, .. }) = res {
                let _ = started.await;
            }
        };
        let _unused = self.spawn(async move {
            tokio::select! {
                () = fut => {},
                () = cancel_token.cancelled() => return,
            };
            drop(event_sender.send(Event::LogReady));
        });
    }

    fn register_file_logging(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        source_fd: OwnedFd,
        file_path: impl AsRef<Path> + Send,
        rotation_config: LogRotationConfig,
    ) {
        let event_sender = event_sender.clone();
        let file_path = file_path.as_ref().to_path_buf();
        let fut = async move {
            let mut ready = false;

            // Open rotating file.
            let mut rotating_file = match RotatingFile::new(file_path.clone(), rotation_config)
                .await
            {
                Ok(rf) => rf,
                Err(err) => {
                    tracing::error!("Failed to open rotating file {}: {err}", file_path.display());
                    return Ok::<(), std::io::Error>(());
                }
            };

            // Prepare to read from `source_fd`.
            let source_fd = AsyncFd::new(source_fd)?;
            let mut buf = [0u8; 8192];

            loop {
                let readable = tokio::select! {
                    readable = source_fd.readable() => readable,
                    () = cancel_token.cancelled() => break,
                };

                let result = match readable {
                    Ok(mut guard) => guard.try_io(|inner| {
                        let fd = inner.get_ref();
                        nix::unistd::read(fd, &mut buf).map_err(io::Error::from)
                    }),
                    Err(err) => {
                        tracing::debug!("AsyncFd readable error: {err}");
                        break;
                    }
                };
                match result {
                    Ok(Ok(0)) => break,
                    Ok(Ok(n)) => {
                        if !ready {
                            ready = true;
                            drop(event_sender.send(Event::LogReady));
                        }
                        // Write to rotating file
                        if let Err(err) = rotating_file.write_all(&buf[..n]).await {
                            tracing::error!("Failed to write to rotating file: {err}");
                            break;
                        }
                    }
                    Ok(Err(err)) => {
                        tracing::debug!("Read error from source fd: {err}");
                        break;
                    }
                    Err(_) => {
                        // Would block - shouldn't happen because readable
                        // indicated ready.
                    }
                }
            }
            Ok(())
        };

        let _unused = self.spawn(async move {
            match fut.await {
                Ok(()) => {}
                Err(err) => tracing::error!("File logging task failed: {err}"),
            }
        });
    }

    fn check_readiness(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        probe: Probe,
    ) {
        let event_sender = event_sender.clone();
        let _unused = self.spawn(async move {
            let ready = tokio::select! {
                ready = probe.check() => ready,
                () = cancel_token.cancelled() => return,
            };
            drop(event_sender.send(Event::ReadinessChecked { ready }));
        });
    }

    fn check_liveness(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        probe: Probe,
    ) {
        let event_sender = event_sender.clone();
        let _unused = self.spawn(async move {
            let should_kill = tokio::select! {
                ready = probe.check() => !ready,
                () = cancel_token.cancelled() => return,
            };
            drop(event_sender.send(Event::LivenessChecked { should_kill }));
        });
    }

    fn schedule(
        &mut self,
        cancel_token: CancellationToken,
        event_sender: &mpsc::UnboundedSender<Event>,
        timeout: Duration,
        event: Event,
    ) {
        let event_sender = event_sender.clone();
        let _unused = self.spawn(async move {
            if tokio::time::timeout(timeout, cancel_token.cancelled()).await.is_err() {
                drop(event_sender.send(event));
            }
        });
    }
}
