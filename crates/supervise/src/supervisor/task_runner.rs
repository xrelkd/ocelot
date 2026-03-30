use std::{os::fd::OwnedFd, time::Duration};

use tokio::{sync::mpsc, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::{
    Reaper,
    reaper::ReapedProcess,
    splice_relay::{Destination, RelayRegistration, SpliceRelay},
    supervisor::{event::Event, probe::Probe},
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
            }
            drop(event_sender.send(Event::LogReady));
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
