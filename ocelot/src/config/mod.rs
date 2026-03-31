mod dependency;
mod error;
mod probe;
mod process;
mod restart;
mod utils;

use std::{collections::HashMap, path::PathBuf, time::Duration};

use ocelot_supervise::{
    LogCompression as SupLogCompression, LogDestination as SupLogDestination,
    LogRotationConfig as SupLogRotationConfig, LogStreamConfig as SupLogStreamConfig,
    supervisor_config, supervisor_probe,
};
use petgraph::{Direction, graph::DiGraph, stable_graph::StableDiGraph};
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use snafu::ResultExt;

pub use self::{
    dependency::DependencyCondition,
    error::{Error, ValidationError},
    probe::{ProbeConfig, ProbeHandlerConfig},
    process::ProcessConfig,
    restart::RestartPolicyConfig,
};
use crate::config::process::{
    LogCompression, LogConfig, LogDestination, LogRotationConfig, LogStreamConfig,
};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupervisorConfig {
    pub version: String,

    #[serde(default = "default_log_level")]
    #[serde_as(as = "DisplayFromStr")]
    pub log_level: tracing::Level,

    #[serde(default)]
    pub processes: HashMap<String, ProcessConfig>,

    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,
}

impl SupervisorConfig {
    const SUPPORTED_VERSION: &'static str = "1.0";

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let orig_path = path.as_ref();
        let path_buf = orig_path.to_owned();
        let Ok(resolved_path) = path_buf.try_resolve() else {
            return Err(Error::ResolveFilePath { file_path: orig_path.to_path_buf() });
        };
        let data = std::fs::read(&resolved_path)
            .with_context(|_| error::OpenConfigSnafu { filename: resolved_path.clone() })?;
        serde_yaml::from_slice(&data).context(error::ParseConfigSnafu { filename: resolved_path })
    }

    /// Converts the `SupervisorConfig` into a vector of
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

impl From<ProcessConfig> for ocelot_supervise::SupervisorConfig {
    fn from(config: ProcessConfig) -> Self {
        let termination_grace_period = config.termination_grace_period;

        let shutdown_signal = config.shutdown_signal.map(|s| s.to_signal());

        let depends_on = config
            .depends_on
            .into_iter()
            .map(|(name, dep)| {
                let condition = dep.condition.map(supervisor_config::DependencyCondition::from);
                (name, supervisor_config::ProcessDependency { condition })
            })
            .collect();

        // Convert environment_variables from Vec<(String, String)> to HashMap<String,
        // String>
        let environment_variables =
            config.environment_variables.into_iter().collect::<HashMap<_, _>>();

        // Map log configuration
        let (log_stdout, log_stderr) = match config.log {
            Some(LogConfig { stdout, stderr }) => {
                (SupLogStreamConfig::from(stdout), SupLogStreamConfig::from(stderr))
            }
            None => (
                SupLogStreamConfig { destination: SupLogDestination::Inherit, rotation: None },
                SupLogStreamConfig { destination: SupLogDestination::Inherit, rotation: None },
            ),
        };

        Self {
            name: String::new(),
            program: PathBuf::from(config.program),
            arguments: config.arguments,
            environment_variables,
            working_directory: config.working_directory.map(PathBuf::from),
            depends_on,
            readiness_probe: config.readiness_probe.map(supervisor_probe::Probe::from),
            liveness_probe: config.liveness_probe.map(supervisor_probe::Probe::from),
            restart_policy: supervisor_config::RestartPolicy::from(
                config.restart_policy.unwrap_or_default(),
            ),
            shutdown_signal,
            termination_grace_period,
            log_stdout,
            log_stderr,
        }
    }
}

impl From<ProbeConfig> for supervisor_probe::Probe {
    fn from(config: ProbeConfig) -> Self {
        let handler = match config.handler {
            ProbeHandlerConfig::HttpGet { host, path, port } => {
                supervisor_probe::ProbeHandler::HttpGet { host, path, port }
            }
            ProbeHandlerConfig::TcpSocket { host, port } => {
                supervisor_probe::ProbeHandler::TcpSocket { host, port }
            }
        };

        Self {
            handler,
            initial_delay: config.initial_delay,
            period: config.period,
            timeout: config.timeout,
            failure_threshold: config.failure_threshold,
            success_threshold: config.success_threshold,
        }
    }
}

impl From<DependencyCondition> for supervisor_config::DependencyCondition {
    fn from(condition: DependencyCondition) -> Self {
        match condition {
            DependencyCondition::Started => Self::Started,
            DependencyCondition::Healthy => Self::Healthy,
            DependencyCondition::Completed => Self::Completed,
            DependencyCondition::CompletedSuccessfully => Self::CompletedSuccessfully,
            DependencyCondition::LogReady => Self::LogReady,
        }
    }
}

impl From<RestartPolicyConfig> for supervisor_config::RestartPolicy {
    fn from(policy: RestartPolicyConfig) -> Self {
        match policy {
            RestartPolicyConfig::Never => Self::Never,
            RestartPolicyConfig::Always { backoff } => {
                Self::Always { backoff: backoff.unwrap_or(Duration::from_secs(2)) }
            }
            RestartPolicyConfig::OnFailure { max_retries, backoff } => Self::OnFailure {
                max_retries: max_retries.unwrap_or(u32::MAX),
                backoff: backoff.unwrap_or(Duration::from_secs(2)),
            },
        }
    }
}

impl From<LogStreamConfig> for SupLogStreamConfig {
    fn from(config: LogStreamConfig) -> Self {
        Self {
            destination: SupLogDestination::from(config.destination),
            rotation: config.rotation.map(SupLogRotationConfig::from),
        }
    }
}

impl From<LogDestination> for SupLogDestination {
    fn from(dest: LogDestination) -> Self {
        match dest {
            LogDestination::Null => Self::Null,
            LogDestination::Inherit => Self::Inherit,
            LogDestination::File { path } => Self::File { path },
        }
    }
}

impl From<LogRotationConfig> for SupLogRotationConfig {
    fn from(config: LogRotationConfig) -> Self {
        Self {
            max_size_bytes: config.max_size_bytes.map(|s| s.as_u64()),
            rotation_interval_secs: config.rotation_interval.map(|d| d.as_secs()),
            max_files: config.max_files,
            max_age_days: config.max_age_days,
            mode: config.mode.and_then(|m| u32::from_str_radix(&m, 8).ok()),
            compression: SupLogCompression::from(config.compression),
        }
    }
}

impl From<LogCompression> for SupLogCompression {
    fn from(compression: LogCompression) -> Self {
        match compression {
            LogCompression::None => Self::None,
            LogCompression::Lz4 => Self::Lz4,
            LogCompression::Gzip => Self::Gzip,
        }
    }
}

const fn default_shutdown_timeout_secs() -> u64 { 60 }

const fn default_log_level() -> tracing::Level { tracing::Level::INFO }

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nix::sys::signal::Signal;
    use ocelot_supervise::{LogDestination, supervisor_config::DependencyCondition};

    use crate::config::{
        Error, ProcessConfig, RestartPolicyConfig, SupervisorConfig, ValidationError,
        dependency::DependencyConfig,
        probe::{ProbeConfig, ProbeHandlerConfig},
        process::ShutdownSignalConfig,
    };

    #[test]
    fn test_shutdown_signal_config_sigterm_explicit() {
        let yaml = r"
type: sigterm
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Sigterm);
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_shutdown_signal_config_name() {
        let yaml = r"
type: name
value: SIGTERM
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config, ShutdownSignalConfig::Name(_)));
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_shutdown_signal_config_number() {
        let yaml = r"
type: number
value: 9
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config, ShutdownSignalConfig::Number(9)));
        assert_eq!(config.to_signal(), Signal::SIGKILL);
    }

    #[test]
    fn test_shutdown_signal_config_invalid_falls_back_to_sigterm() {
        let yaml = r"
type: name
value: INVALID
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_restart_policy_never() {
        let yaml = r"type: Never";
        let policy: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(policy, RestartPolicyConfig::Never);
    }

    #[test]
    fn test_restart_policy_always_with_backoff() {
        let yaml = r"
type: Always
backoff: 5s
";
        let policy: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(policy, RestartPolicyConfig::Always { backoff: Some(Duration::from_secs(5)) });
    }

    #[test]
    fn test_restart_policy_on_failure_full() {
        let yaml = r"
type: OnFailure
maxRetries: 10
backoff: 3s
";
        let policy: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            policy,
            RestartPolicyConfig::OnFailure {
                max_retries: Some(10),
                backoff: Some(Duration::from_secs(3))
            }
        );
    }

    #[test]
    fn test_process_config_to_supervisor_config() {
        let yaml = r"
program: /usr/bin/test
terminationGracePeriod: 30s
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisor_config: ocelot_supervise::SupervisorConfig = config.into();
        assert_eq!(supervisor_config.program, std::path::PathBuf::from("/usr/bin/test"));
        assert_eq!(supervisor_config.termination_grace_period.as_secs(), 30);
    }

    #[test]
    fn test_process_config_with_signal() {
        let yaml = r"
program: /usr/bin/test
shutdownSignal:
  type: number
  value: 9
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisor_config: ocelot_supervise::SupervisorConfig = config.into();
        assert_eq!(supervisor_config.shutdown_signal, Some(Signal::SIGKILL));
    }

    #[test]
    fn test_process_config_with_env() {
        let yaml = r"
program: /usr/bin/test
environmentVariables:
  FOO: bar
  BAZ: qux
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.program, "/usr/bin/test");
        assert!(config.arguments.is_empty());
        assert_eq!(config.get_env("FOO"), Some(&"bar".to_string()));
        assert_eq!(config.environment_variables.len(), 2);
    }

    #[test]
    fn test_process_config_with_depends_on() {
        let yaml = r"
program: /usr/bin/test
dependsOn:
  db:
    condition: Healthy
  cache:
    condition: Started
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisor_config: ocelot_supervise::SupervisorConfig = config.into();
        assert_eq!(supervisor_config.depends_on.len(), 2);
        assert!(supervisor_config.depends_on.contains_key("db"));
        assert!(supervisor_config.depends_on.contains_key("cache"));
    }

    #[test]
    fn test_supervisor_config_minimal() {
        let yaml = r#"
version: "1.0"
processes: {}
"#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, "1.0");
        assert!(config.processes.is_empty());
        assert_eq!(config.shutdown_timeout_secs, 60);
    }

    #[test]
    fn test_supervisor_config_with_processes() {
        let yaml = r#"
version: "1.0"
shutdownTimeoutSecs: 120
processes:
  app:
    program: /usr/bin/app
"#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.shutdown_timeout_secs, 120);
        assert_eq!(config.processes.len(), 1);
        assert!(config.processes.contains_key("app"));
    }

    #[test]
    fn test_template_basic_is_serializable() {
        let yaml_bytes = SupervisorConfig::template_basic();
        let yaml_str = String::from_utf8(yaml_bytes).expect("template is valid UTF-8");
        let config: SupervisorConfig =
            serde_yaml::from_str(&yaml_str).expect("template is valid YAML");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.processes.len(), 4);
        assert!(config.processes.contains_key("nginx"));
        assert!(config.processes.contains_key("myapp"));
        assert!(config.processes.contains_key("redis"));
        assert!(config.processes.contains_key("postgres"));

        let nginx = &config.processes["nginx"];
        assert_eq!(nginx.program, "/usr/sbin/nginx");
        assert!(nginx.arguments.contains(&"-g".to_string()));

        let redis = &config.processes["redis"];
        let shutdown_signal = redis.shutdown_signal.as_ref().expect("redis should have signal");
        assert!(matches!(shutdown_signal, ShutdownSignalConfig::Number(9)));
    }

    #[test]
    fn test_probe_config_default_handler() {
        let yaml = "{}";
        let config: ProbeConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config.handler, ProbeHandlerConfig::HttpGet { .. }));
    }

    #[test]
    fn test_probe_config_tcp_socket() {
        let yaml = r"
        handler:
          type: tcpSocket
          port: 5432
        initialDelay: 10s
        ";
        let config: ProbeConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(config.handler, ProbeHandlerConfig::TcpSocket { port: 5432, .. }));
        assert_eq!(config.initial_delay, Duration::from_secs(10));
    }

    #[test]
    fn test_dependency_config_conditions() {
        let yaml = r"
condition: Healthy
";
        let config: DependencyConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.condition.is_some());
    }

    #[test]
    fn test_dependency_config_no_condition() {
        let yaml = "{}";
        let config: DependencyConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.condition.is_none());
    }

    #[test]
    fn test_validate_cycle_detection() {
        let yaml = r#"
version: "1.0"
processes:
  a:
    program: /usr/bin/a
    dependsOn:
      b:
        condition: Started
  b:
    program: /usr/bin/b
    dependsOn:
      a:
        condition: Started
"#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let yaml = r#"
version: "1.0"
processes:
  a:
    program: /usr/bin/a
    dependsOn:
      nonexistent:
        condition: Started
"#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_valid_dependencies() {
        let yaml = r#"
version: "1.0"
processes:
  a:
    program: /usr/bin/a
    dependsOn:
      b:
        condition: Started
  b:
    program: /usr/bin/b
"#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_to_supervisors_empty() {
        let yaml = r#"
        version: "1.0"
        processes: {}
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisors = config.to_supervisors();
        assert!(supervisors.is_empty());
    }

    #[test]
    fn test_to_supervisors_no_deps() {
        let yaml = r#"
        version: "1.0"
        processes:
          app1:
            program: /usr/bin/app1
          app2:
            program: /usr/bin/app2
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisors = config.to_supervisors();
        assert_eq!(supervisors.len(), 2);
        // Verify each has empty depends_on
        for sup in supervisors {
            assert!(sup.depends_on.is_empty());
        }
    }

    #[test]
    fn test_to_supervisors_with_deps() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            dependsOn:
              db:
                condition: Healthy
              cache:
                condition: Started
          db:
            program: /usr/bin/db
          cache:
            program: /usr/bin/cache
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let mut supervisors = config.to_supervisors();
        // Sort by name for deterministic check
        supervisors.sort_by_key(|s| s.name.clone());

        // Find app's config
        let app_sup = supervisors.iter().find(|s| s.name == "app").unwrap();
        assert_eq!(app_sup.depends_on.len(), 2);
        assert!(app_sup.depends_on.contains_key("db"));
        assert!(app_sup.depends_on.contains_key("cache"));
        // Check conditions
        assert!(matches!(
            app_sup.depends_on.get("db").unwrap().condition,
            Some(DependencyCondition::Healthy)
        ));
        assert!(matches!(
            app_sup.depends_on.get("cache").unwrap().condition,
            Some(DependencyCondition::Started)
        ));

        // db and cache should have empty depends_on
        let db_sup = supervisors.iter().find(|s| s.name == "db").unwrap();
        assert!(db_sup.depends_on.is_empty());
        let cache_sup = supervisors.iter().find(|s| s.name == "cache").unwrap();
        assert!(cache_sup.depends_on.is_empty());
    }

    #[test]
    fn test_backward_compatibility_no_log_field() {
        // Verify that configs without the optional log field default to Inherit for
        // both streams
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let supervisors = config.to_supervisors();
        let app_sup = supervisors.iter().find(|s| s.name == "app").unwrap();

        // Without log config, both stdout and stderr should default to Inherit
        assert!(matches!(app_sup.log_stdout.destination, LogDestination::Inherit));
        assert!(matches!(app_sup.log_stderr.destination, LogDestination::Inherit));
        assert!(app_sup.log_stdout.rotation.is_none());
        assert!(app_sup.log_stderr.rotation.is_none());
    }

    #[test]
    fn test_validate_missing_program() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: ""
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::MissingProcessProgram { .. }));
                if let ValidationError::MissingProcessProgram { process } = source {
                    assert_eq!(process, "app");
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_termination_grace_period() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            terminationGracePeriod: 0s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidTerminationGracePeriod { .. }));
                if let ValidationError::InvalidTerminationGracePeriod { value } = source {
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_log_rotation_max_size_zero() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            log:
              stdout:
                destination:
                  type: file
                  path: /var/log/app/stdout.log
                rotation:
                  maxSizeBytes: 0
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidLogRotation { .. }));
                if let ValidationError::InvalidLogRotation { field, value } = source {
                    assert_eq!(field, "stdout.maxSizeBytes");
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_log_rotation_interval_zero() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            log:
              stdout:
                destination:
                  type: file
                  path: /var/log/app/stdout.log
                rotation:
                  rotationInterval: 0s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidLogRotation { .. }));
                if let ValidationError::InvalidLogRotation { field, value } = source {
                    assert_eq!(field, "stdout.rotationInterval");
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_log_rotation_max_files_zero() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            log:
              stdout:
                destination:
                  type: file
                  path: /var/log/app/stdout.log
                rotation:
                  maxFiles: 0
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidLogRotation { .. }));
                if let ValidationError::InvalidLogRotation { field, value } = source {
                    assert_eq!(field, "stdout.maxFiles");
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_log_rotation_max_age_zero() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            log:
              stdout:
                destination:
                  type: file
                  path: /var/log/app/stdout.log
                rotation:
                  maxAgeDays: 0
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidLogRotation { .. }));
                if let ValidationError::InvalidLogRotation { field, value } = source {
                    assert_eq!(field, "stdout.maxAgeDays");
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_log_rotation_both_zero() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            log:
              stdout:
                destination:
                  type: file
                  path: /var/log/app/stdout.log
                rotation:
                  maxSizeBytes: 0
                  rotationInterval: 0s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        // When both are zero, the first check (maxSizeBytes == 0) catches it
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidLogRotation { .. }));
                if let ValidationError::InvalidLogRotation { field, value } = source {
                    assert_eq!(field, "stdout.maxSizeBytes");
                    assert_eq!(value, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_probe_timeout() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            readinessProbe:
              handler:
                type: httpGet
                path: /health
                port: 8080
              initialDelay: 5s
              period: 10s
              timeout: 15s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidProbeTimeout { .. }));
                if let ValidationError::InvalidProbeTimeout { timeout, period } = source {
                    assert_eq!(timeout, 15);
                    assert_eq!(period, 10);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_probe_port() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            readinessProbe:
              handler:
                type: httpGet
                path: /health
                port: 0
              period: 10s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidProbePort { .. }));
                if let ValidationError::InvalidProbePort { port } = source {
                    assert_eq!(port, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_invalid_restart_backoff() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            restartPolicy:
              type: Always
              backoff: 0s
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::InvalidRestartBackoff { .. }));
                if let ValidationError::InvalidRestartBackoff { backoff } = source {
                    assert_eq!(backoff, 0);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_duplicate_environment_variables() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            environmentVariables:
              FOO: bar
              FOO: baz
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::DuplicateEnvironmentVariables { .. }));
                if let ValidationError::DuplicateEnvironmentVariables { process, variables } =
                    source
                {
                    assert_eq!(process, "app");
                    assert_eq!(variables, vec!["FOO"]);
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_cycle_detection_full_path() {
        let yaml = r#"
        version: "1.0"
        processes:
          a:
            program: /usr/bin/a
            dependsOn:
              b:
                condition: Started
          b:
            program: /usr/bin/b
            dependsOn:
              c:
                condition: Started
          c:
            program: /usr/bin/c
            dependsOn:
              a:
                condition: Started
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        let err = config.validate().unwrap_err();
        match err {
            Error::Validate { source } => {
                assert!(matches!(source, ValidationError::CyclicDependency { .. }));
                if let ValidationError::CyclicDependency { cycle } = source {
                    // The cycle should contain all three nodes in order
                    assert!(cycle.len() >= 3);
                    assert!(cycle.contains(&"a".to_string()));
                    assert!(cycle.contains(&"b".to_string()));
                    assert!(cycle.contains(&"c".to_string()));
                }
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_validate_successful_config() {
        let yaml = r#"
        version: "1.0"
        processes:
          app:
            program: /usr/bin/app
            terminationGracePeriod: 30s
            environmentVariables:
              FOO: bar
              BAZ: qux
        "#;
        let config: SupervisorConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_ok());
    }
}
