mod config;
mod event;
mod executor;

use std::collections::HashMap;

pub use config::Config as OrchestratorConfig;
use tokio::sync::{mpsc, oneshot};

pub use self::executor::Executor as OrchestratorExecutor;
use crate::supervisor::ProcessStatus;

#[derive(Clone)]
pub struct Orchestrator {
    event_sender: mpsc::UnboundedSender<event::Event>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> (Self, OrchestratorExecutor) {
        let (executor, event_sender) = OrchestratorExecutor::new(config);
        (Self { event_sender }, executor)
    }

    // TODO: Use this function.
    #[expect(
        dead_code,
        reason = "Kept for future dynamic control capabilities as indicated by TODO comment"
    )]
    #[tracing::instrument(name = "Orchestrator::stop_supervisor", skip_all)]
    pub async fn stop_supervisor(&self, name: impl Into<String>) -> bool {
        let (sender, receiver) = oneshot::channel();
        drop(
            self.event_sender
                .send(event::Event::StopSupervisor { name: name.into(), resp: sender }),
        );
        receiver.await.unwrap_or(false)
    }

    // TODO: Use this function.
    #[expect(
        dead_code,
        reason = "Kept for future dynamic control capabilities as indicated by TODO comment"
    )]
    #[tracing::instrument(name = "Orchestrator::restart_supervisor", skip_all)]
    pub async fn restart_supervisor(&self, name: impl Into<String>) -> bool {
        let (sender, receiver) = oneshot::channel();
        drop(
            self.event_sender
                .send(event::Event::RestartSupervisor { name: name.into(), resp: sender }),
        );
        receiver.await.unwrap_or(false)
    }

    // TODO: Use this function.
    #[expect(
        dead_code,
        reason = "Kept for future dynamic control capabilities as indicated by TODO comment"
    )]
    #[tracing::instrument(name = "Orchestrator::get_all_statuses", skip_all)]
    pub async fn get_all_statuses(&self) -> HashMap<String, ProcessStatus> {
        let (sender, receiver) = oneshot::channel();
        drop(self.event_sender.send(event::Event::GetAllStatuses { resp: sender }));
        receiver.await.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        orchestrator::config::Config,
        supervisor::{
            LogDestination, LogStreamConfig,
            config::{Config as SupervisorConfig, RestartPolicy},
        },
    };

    #[test]
    fn test_orchestrator_config_default() {
        let config = Config::default();
        assert!(config.supervisors.is_empty());
        assert_eq!(config.shutdown_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_orchestrator_config_with_supervisors() {
        let supervisor_config = SupervisorConfig {
            name: "test".to_string(),
            program: std::path::PathBuf::from("/bin/sleep"),
            arguments: vec!["3600".to_string()],
            environment_variables: std::collections::HashMap::new(),
            working_directory: None,
            depends_on: std::collections::HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
            log_stdout: LogStreamConfig { destination: LogDestination::Inherit, rotation: None },
            log_stderr: LogStreamConfig { destination: LogDestination::Inherit, rotation: None },
        };

        let config = Config {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(60),
        };

        assert_eq!(config.supervisors.len(), 1);
        assert_eq!(config.supervisors[0].name, "test");
        assert_eq!(config.shutdown_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_orchestrator_config_clone() {
        let config = Config { supervisors: Vec::new(), shutdown_timeout: Duration::from_secs(45) };

        let cloned = config.clone();
        assert_eq!(cloned.shutdown_timeout, config.shutdown_timeout);
    }
}
