use crate::reaper::RegisteredProcess;

pub enum Event {
    Shutdown,
    ReapProcess,
    RegisterProcess { registered_process: RegisteredProcess },
}
