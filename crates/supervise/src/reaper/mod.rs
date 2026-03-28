mod event;
mod executor;

use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

pub use self::executor::Executor as ReaperExecutor;

#[derive(Clone)]
pub struct Reaper {
    register_sender: mpsc::UnboundedSender<RegisteredProcess>,
}

impl Reaper {
    #[must_use]
    pub fn new() -> (Self, ReaperExecutor) {
        let (register_sender, register_receiver) = mpsc::unbounded_channel();
        (Self { register_sender }, ReaperExecutor::new(register_receiver))
    }

    #[tracing::instrument(name = "Reaper::register", skip_all)]
    pub fn register(
        &self,
        pid: nix::unistd::Pid,
        termination_grace_period: Duration,
    ) -> oneshot::Receiver<ReapedProcess> {
        let (sender, receiver) = oneshot::channel();

        let _unused =
            self.register_sender.send(RegisteredProcess { pid, termination_grace_period, sender });
        receiver
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReapedProcess {
    pub pid: nix::unistd::Pid,
    pub exit_code: i32,
}

pub struct RegisteredProcess {
    pub pid: nix::unistd::Pid,
    pub termination_grace_period: Duration,
    pub sender: oneshot::Sender<ReapedProcess>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nix::unistd::Pid;

    use crate::reaper::{ReapedProcess, Reaper, RegisteredProcess};

    #[test]
    fn test_reaped_process() {
        let reaped = ReapedProcess { pid: Pid::from_raw(1234), exit_code: 0 };
        assert_eq!(reaped.pid, Pid::from_raw(1234));
        assert_eq!(reaped.exit_code, 0);
    }

    #[test]
    fn test_reaped_process_eq() {
        let reaped1 = ReapedProcess { pid: Pid::from_raw(1234), exit_code: 0 };
        let reaped2 = ReapedProcess { pid: Pid::from_raw(1234), exit_code: 0 };
        let reaped3 = ReapedProcess { pid: Pid::from_raw(5678), exit_code: 0 };

        assert_eq!(reaped1, reaped2);
        assert_ne!(reaped1, reaped3);
    }

    #[test]
    fn test_reaper_new() {
        let (reaper, _executor) = Reaper::new();
        let (sender, _rx) = tokio::sync::oneshot::channel();
        let process = RegisteredProcess {
            pid: Pid::from_raw(1),
            sender,
            termination_grace_period: Duration::from_secs(5),
        };
        assert!(reaper.register_sender.send(process).is_ok());
    }
}
