use tokio::sync::oneshot;

use crate::supervisor::ProcessStatus;

#[derive(Debug)]
pub enum Event {
    Shutdown,
    StopSupervisor { name: String, resp: oneshot::Sender<bool> },
    RestartSupervisor { name: String, resp: oneshot::Sender<bool> },
    GetAllStatuses { resp: oneshot::Sender<std::collections::HashMap<String, ProcessStatus>> },
}
