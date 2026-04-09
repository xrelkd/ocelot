use std::{collections::HashMap, time::Duration};

use serde::Deserialize;

use crate::{
    config::{error::Error, supervise::ProcessConfig},
    graph::DiGraph,
};

/// Bootstrap supervise configuration wrapper.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BootstrapSuperviseConfig {
    /// Process definitions for supervise.
    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,

    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

impl BootstrapSuperviseConfig {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        self.check_missing_dependencies()?;
        self.detect_dependency_cycles()?;
        self.validate_process_configs()?;
        Ok(())
    }

    fn check_missing_dependencies(&self) -> Result<(), Error> {
        for (name, config) in &self.processes {
            for dep_name in config.depends_on.keys() {
                snafu::ensure!(
                    self.processes.contains_key(dep_name),
                    crate::config::error::MissingDependencySnafu {
                        process: name.clone(),
                        depends_on: dep_name.clone()
                    }
                );
            }
        }
        Ok(())
    }

    fn detect_dependency_cycles(&self) -> Result<(), Error> {
        let mut graph = DiGraph::<String>::new();

        for name in self.processes.keys() {
            let _ = graph.add_node(name, name.clone());
        }

        for (name, config) in &self.processes {
            for dep_name in config.depends_on.keys() {
                graph.add_edge(name, dep_name);
            }
        }

        graph.detect_cycle().map_or(Ok(()), |cycle| {
            crate::config::error::CyclicDependencySnafu { cycle }.fail().map_err(Error::from)
        })
    }

    fn validate_process_configs(&self) -> Result<(), Error> {
        for (name, process_config) in &self.processes {
            process_config.validate(name)?;
        }
        Ok(())
    }
}

impl From<BootstrapSuperviseConfig> for ocelot_supervise::OrchestratorConfig {
    fn from(supervise: BootstrapSuperviseConfig) -> Self {
        Self {
            supervisors: supervise
                .processes
                .into_iter()
                .map(|(name, process)| {
                    let mut config = ocelot_supervise::SupervisorConfig::from(process);
                    config.name = name;
                    config
                })
                .collect(),
            shutdown_timeout: Duration::from_secs(supervise.shutdown_timeout_secs),
        }
    }
}

const fn default_shutdown_timeout() -> u64 { 30 }
