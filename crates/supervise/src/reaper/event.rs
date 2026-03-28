use tokio::sync::oneshot;

use crate::ReapedProcess;

pub enum Event {
    Shutdown,
    ReapProcess,
    RegisterProcess { pid: nix::unistd::Pid, sender: oneshot::Sender<ReapedProcess> },
}
