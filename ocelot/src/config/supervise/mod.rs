mod dependency;
mod probe;
mod process;
mod restart;

#[cfg(test)]
mod tests;

use std::{collections::HashMap, path::Path};

use ocelot_supervise::supervisor_config;
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;
use tracing::Level;

pub use self::process::ProcessConfig;
use crate::{
    config::{error, error::Error},
    graph::DiGraph,
};

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

    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let orig_path = path.as_ref();
        let path_buf = orig_path.to_owned();
        let Ok(resolved_path) = path_buf.try_resolve() else {
            return error::ResolveFilePathSnafu { file_path: orig_path.to_path_buf() }.fail();
        };
        let data = std::fs::read(&resolved_path)
            .with_context(|_| error::OpenConfigSnafu { filename: resolved_path.clone() })?;
        serde_yaml::from_slice(&data).context(error::ParseConfigSnafu { filename: resolved_path })
    }

    /// Converts the `SuperviseConfig` into a vector of
    /// `ocelot_supervise::SupervisorConfig` by transforming each process
    /// configuration and resolving dependencies using a directed graph.
    ///
    /// This method builds a `DiGraph` where nodes represent processes and
    /// edges represent dependency relationships (from dependency to
    /// dependent). For each process, its `depends_on` list is populated by
    /// querying the graph's incoming neighbors, ensuring all direct
    /// dependencies are correctly listed with their original conditions.
    pub fn to_supervisors(&self) -> Vec<ocelot_supervise::SupervisorConfig> {
        let mut graph = DiGraph::<String>::new();

        for name in self.processes.keys() {
            let _ = graph.add_node(name, name.clone());
        }

        for (name, config) in &self.processes {
            for dep_name in config.depends_on.keys() {
                graph.add_edge(name, dep_name);
            }
        }

        self.processes
            .iter()
            .map(|(name, config)| {
                let mut supervisor = ocelot_supervise::SupervisorConfig::from(config.clone());
                supervisor.name.clone_from(name);
                supervisor.depends_on = {
                    let name_id = graph.get_id(name);
                    name_id
                        .map(|id| {
                            graph
                                .neighbors(id)
                                .iter()
                                .filter_map(|&neighbor_id| {
                                    graph.get_label(neighbor_id).and_then(|dep_name| {
                                        let dep_name_str = dep_name.clone();
                                        config.depends_on.get(&dep_name_str).map(|dep_config| {
                                            let condition = dep_config
                                                .condition
                                                .map(supervisor_config::DependencyCondition::from);
                                            (
                                                dep_name_str,
                                                supervisor_config::ProcessDependency { condition },
                                            )
                                        })
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default()
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
        snafu::ensure!(
            self.version == Self::SUPPORTED_VERSION,
            error::InvalidVersionSnafu { version: self.version.clone() }
        );
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
                snafu::ensure!(
                    self.processes.contains_key(dep_name),
                    error::MissingDependencySnafu {
                        process: name.clone(),
                        depends_on: dep_name.clone()
                    }
                );
            }
        }
        Ok(())
    }

    /// Detects circular dependencies using Kosaraju's algorithm.
    ///
    /// Time complexity: O(P + D) where P = number of processes, D = total
    /// dependencies. Space complexity: O(P + D) for the graph and index
    /// map.
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
            error::CyclicDependencySnafu { cycle }.fail().map_err(Error::from)
        })
    }

    pub fn template_minimal() -> Vec<u8> { include_bytes!("templates/minimal.yaml").to_vec() }

    pub fn template_basic() -> Vec<u8> { include_bytes!("templates/basic.yaml").to_vec() }

    pub fn template_full() -> Vec<u8> { include_bytes!("templates/full.yaml").to_vec() }
}
