use std::collections::HashMap;

use ocelot_supervise::supervisor_config;
use petgraph::{Direction, graph::DiGraph, stable_graph::StableDiGraph};
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;
use tracing::Level;

use super::{Error, ProcessConfig, ValidationError};
use crate::config::utils;

const fn default_shutdown_timeout_secs() -> u64 { 60 }

const fn default_log_level() -> Level { Level::INFO }

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuperviseConfig {
    pub version: String,

    #[serde(default = "default_log_level")]
    #[serde_as(as = "DisplayFromStr")]
    pub log_level: Level,

    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,

    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,
}

impl SuperviseConfig {
    const SUPPORTED_VERSION: &'static str = "1.0";

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let orig_path = path.as_ref();
        let path_buf = orig_path.to_owned();
        let Ok(resolved_path) = path_buf.try_resolve() else {
            return Err(Error::ResolveFilePath { file_path: orig_path.to_path_buf() });
        };
        let data = std::fs::read(&resolved_path)
            .with_context(|_| super::error::OpenConfigSnafu { filename: resolved_path.clone() })?;
        serde_yaml::from_slice(&data)
            .context(super::error::ParseConfigSnafu { filename: resolved_path })
    }

    /// Converts the `SuperviseConfig` into a vector of
    /// `ocelot_supervise::SupervisorConfig` by transforming each process
    /// configuration and resolving dependencies using a directed graph.
    ///
    /// This method builds a `StableDiGraph` where nodes represent processes and
    /// edges represent dependency relationships (from dependency to
    /// dependent). For each process, its `depends_on` list is populated by
    /// querying the graph's incoming neighbors, ensuring all direct
    /// dependencies are correctly listed with their original conditions.
    pub fn to_supervisors(&self) -> Vec<ocelot_supervise::SupervisorConfig> {
        let mut graph = StableDiGraph::<String, ()>::new();
        let name_to_node = self
            .processes
            .keys()
            .map(|name| (name.clone(), graph.add_node(name.clone())))
            .collect::<HashMap<_, _>>();

        for (name, config) in &self.processes {
            let name_idx = name_to_node[name];
            for dep_name in config.depends_on.keys() {
                if let Some(&dep_idx) = name_to_node.get(dep_name) {
                    let _ = graph.add_edge(dep_idx, name_idx, ());
                }
            }
        }

        self.processes
            .iter()
            .map(|(name, config)| {
                let mut supervisor = ocelot_supervise::SupervisorConfig::from(config.clone());
                supervisor.name.clone_from(name);
                supervisor.depends_on = {
                    let node_idx = name_to_node[name];
                    graph
                        .neighbors_directed(node_idx, Direction::Incoming)
                        .filter_map(|neighbor_idx| {
                            let dep_name = graph[neighbor_idx].clone();
                            config.depends_on.get(&dep_name).map(|dep_config| {
                                let condition = dep_config
                                    .condition
                                    .map(supervisor_config::DependencyCondition::from);
                                (dep_name, supervisor_config::ProcessDependency { condition })
                            })
                        })
                        .collect()
                };
                supervisor
            })
            .collect()
    }

    /// Validates all processes in the configuration.
    pub fn validate(&self) -> Result<(), Error> {
        self.check_version()?;
        self.check_missing_dependencies()?;
        self.detect_dependency_cycles()?;
        self.validate_process_configs()?;

        Ok(())
    }

    /// Checks that the config version is supported.
    fn check_version(&self) -> Result<(), Error> {
        if self.version != Self::SUPPORTED_VERSION {
            return Err(Error::Validate {
                source: ValidationError::InvalidVersion { version: self.version.clone() },
            });
        }
        Ok(())
    }

    /// Validate each process configuration.
    fn validate_process_configs(&self) -> Result<(), Error> {
        for (name, config) in &self.processes {
            config.validate(name)?;
        }
        Ok(())
    }

    /// Checks that all declared dependencies reference existing processes.
    ///
    /// Time complexity: O(P * D) where P = number of processes, D = max
    /// dependencies per process. Space complexity: O(1) additional space.
    fn check_missing_dependencies(&self) -> Result<(), Error> {
        for (name, config) in &self.processes {
            for dep_name in config.depends_on.keys() {
                if !self.processes.contains_key(dep_name) {
                    return Err(Error::Validate {
                        source: ValidationError::MissingDependency {
                            process: name.clone(),
                            depends_on: dep_name.clone(),
                        },
                    });
                }
            }
        }
        Ok(())
    }

    /// Detects circular dependencies using topological sort (Kahn's algorithm)
    /// and extracts full cycle path using Kosaraju's algorithm + DFS.
    ///
    /// Time complexity: O(P + D) where P = number of processes, D = total
    /// dependencies. Space complexity: O(P + D) for the graph and index
    /// map.
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
            // Get strongly connected components
            let sccs = petgraph::algo::kosaraju_scc(&graph);
            let node_name = graph[node].clone();
            // Find the SCC containing the failing node
            let scc_opt = sccs.iter().find(|scc| scc.contains(&node)).cloned();
            let Some(scc) = scc_opt else {
                // Should not happen: toposort failed, node must be in a cycle SCC
                return Err(Error::Validate {
                    source: ValidationError::CyclicDependency { cycle: vec![node_name] },
                });
            };
            // Extract a cycle from this SCC using DFS
            utils::find_cycle_in_scc(&graph, &scc, node).map_or_else(
                || {
                    Err(Error::Validate {
                        source: ValidationError::CyclicDependency { cycle: vec![node_name] },
                    })
                },
                |cycle_nodes| {
                    let cycle = cycle_nodes.into_iter().map(|idx| graph[idx].clone()).collect();
                    Err(Error::Validate { source: ValidationError::CyclicDependency { cycle } })
                },
            )
        } else {
            Ok(())
        }
    }

    pub fn template_basic() -> Vec<u8> { include_bytes!("templates/basic.yaml").to_vec() }
}
