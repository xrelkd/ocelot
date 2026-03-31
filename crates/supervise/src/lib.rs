mod command;
mod error;
mod orchestrator;
mod reaper;
mod rotating_file;
mod splice_relay;
mod supervisor;

use nix::unistd;
use snafu::ResultExt;

pub use self::{
    command::Command,
    error::Error,
    orchestrator::OrchestratorConfig,
    reaper::{ReapedProcess, Reaper, ReaperExecutor},
    splice_relay::{
        Builder as SpliceRelayBuilder, Config as SpliceRelayConfig, Error as SpliceRelayError,
        RelayEntry, SpliceRelay, Status as RelayStatus,
    },
    supervisor::{
        DependencyRegistry, LogCompression, LogDestination, LogRotationConfig, LogStreamConfig,
        Phase, ProcessStatus, RestartPolicy, Supervisor, SupervisorConfig, SupervisorExecutor,
        config as supervisor_config,
        config::{DependencyCondition, ProcessDependency},
        probe as supervisor_probe,
        probe::{Probe, ProbeHandler},
    },
};
use crate::orchestrator::Orchestrator;

/// Executes the supervisor orchestrator with the given configuration.
///
/// Initializes a Tokio runtime and runs the orchestrator which manages
/// a collection of process supervisors until shutdown completion.
///
/// # Arguments
///
/// * `supervisors` - Configuration for each process to supervise
///
/// # Returns
///
/// Returns the exit code of the child process. If the child was terminated by a
/// signal, returns `128 + signal_number` (following Unix convention).
///
/// # Errors
///
/// Returns an error if the Tokio runtime cannot be initialized.
///
/// # Panics
///
/// This function should not panic under normal operation.
pub fn execute(config: OrchestratorConfig) -> Result<i32, Error> {
    let pid = unistd::getpid();

    if pid.as_raw() == 1 {
        tracing::info!("Start with PID 1");
    } else {
        tracing::warn!("Entry should be the first process (PID 1), current PID: {pid}");
    }

    let runtime = tokio::runtime::Runtime::new().context(error::InitializeTokioRuntimeSnafu)?;
    let (_orchestrator, orchestrator_executor) = Orchestrator::new(config);
    runtime.block_on(orchestrator_executor.serve())?;
    Ok(0)
}
