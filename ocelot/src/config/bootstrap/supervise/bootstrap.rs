use std::{collections::HashMap, time::Duration};

use petgraph::graph::DiGraph;
use serde::Deserialize;

use crate::config::{error::Error, supervise::ProcessConfig, utils};

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
        let mut graph = DiGraph::<String, ()>::new();
        let mut indices = HashMap::new();

        for name in self.processes.keys() {
            let _ = indices.insert(name.clone(), graph.add_node(name.clone()));
        }

        for (name, config) in &self.processes {
            let from = indices[name];
            for dep_name in config.depends_on.keys() {
                let to = indices[dep_name];
                let _ = graph.add_edge(from, to, ());
            }
        }

        if let Err(cycle) = petgraph::algo::toposort(&graph, None) {
            let node = cycle.node_id();
            let sccs = petgraph::algo::kosaraju_scc(&graph);
            let node_name = graph[node].clone();
            let scc_opt = sccs.iter().find(|scc| scc.contains(&node)).cloned();
            let Some(scc) = scc_opt else {
                return crate::config::error::CyclicDependencySnafu { cycle: vec![node_name] }
                    .fail()
                    .map_err(Error::from);
            };
            utils::find_cycle_in_scc(&graph, &scc, node).map_or_else(
                || {
                    crate::config::error::CyclicDependencySnafu { cycle: vec![node_name] }
                        .fail()
                        .map_err(Error::from)
                },
                |cycle_nodes| {
                    let cycle: Vec<String> =
                        cycle_nodes.iter().map(|&n| graph[n].clone()).collect();
                    crate::config::error::CyclicDependencySnafu { cycle }
                        .fail()
                        .map_err(Error::from)
                },
            )
        } else {
            Ok(())
        }
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
