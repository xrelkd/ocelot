use tokio::sync::oneshot;

use crate::supervisor::ProcessStatus;

#[derive(Debug)]
pub enum Event {
    Start,
    Shutdown,
    ProcessReaped { exit_code: i32 },
    CheckReadiness,
    CheckLiveness,
    ReadinessChecked { ready: bool },
    LivenessChecked { should_kill: bool },
    GetStatus { resp: oneshot::Sender<ProcessStatus> },
    LogReady,
}
