mod dependency;
mod error;
mod probe;
mod process;
mod restart;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    time::Duration,
};

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
use crate::config::process::{LogCompression, LogConfig, LogDestination, LogStreamConfig};

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

    pub fn template_basic() -> Vec<u8> { include_bytes!("templates/basic.yaml").to_vec() }

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
                        .collect::<HashMap<_, _>>()
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
        // Validate each process configuration
        for (name, config) in &self.processes {
            self.validate_process_config(name, config)?;
        }
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

    #[allow(
        clippy::too_many_lines,
        clippy::unused_self,
        clippy::collapsible_if,
        clippy::uninlined_format_args,
        clippy::unnecessary_map_or
    )]
    fn validate_process_config(&self, name: &str, config: &ProcessConfig) -> Result<(), Error> {
        // 2.2: Check program is non-empty
        if config.program.is_empty() {
            return Err(Error::Validate {
                source: ValidationError::MissingProcessProgram { process: name.to_string() },
            });
        }

        // 2.3: Check termination_grace_period > 0
        if config.termination_grace_period.is_zero() {
            return Err(Error::Validate {
                source: ValidationError::InvalidTerminationGracePeriod {
                    value: config.termination_grace_period.as_secs(),
                },
            });
        }

        // 2.3: Check termination_grace_period > 0
        if config.termination_grace_period.is_zero() {
            return Err(Error::Validate {
                source: ValidationError::InvalidTerminationGracePeriod {
                    value: config.termination_grace_period.as_secs(),
                },
            });
        }

        // 2.4: Validate log rotation parameters
        if let Some(log) = &config.log {
            for (stream_name, stream_config) in [("stdout", &log.stdout), ("stderr", &log.stderr)] {
                if let Some(rotation) = &stream_config.rotation {
                    // Validate each rotation field is positive if Some
                    if let Some(max_size) = rotation.max_size_bytes {
                        if max_size.as_u64() == 0 {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidLogRotation {
                                    field: format!("{}.maxSizeBytes", stream_name),
                                    value: 0,
                                },
                            });
                        }
                    }
                    if let Some(interval) = rotation.rotation_interval {
                        if interval.is_zero() {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidLogRotation {
                                    field: format!("{}.rotationInterval", stream_name),
                                    value: 0,
                                },
                            });
                        }
                    }
                    if let Some(max_files) = rotation.max_files {
                        if max_files == 0 {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidLogRotation {
                                    field: format!("{}.maxFiles", stream_name),
                                    value: 0,
                                },
                            });
                        }
                    }
                    if let Some(max_age) = rotation.max_age_days {
                        if max_age == 0 {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidLogRotation {
                                    field: format!("{}.maxAgeDays", stream_name),
                                    value: 0,
                                },
                            });
                        }
                    }
                    // 2.4: Ensure at least one of max_size_bytes or rotation_interval_secs is
                    // positive
                    let has_size = rotation.max_size_bytes.map_or(false, |s| s.as_u64() > 0);
                    let has_interval =
                        rotation.rotation_interval.map_or(false, |d| d.as_secs() > 0);
                    if !has_size && !has_interval {
                        return Err(Error::Validate {
                            source: ValidationError::InvalidRotationConfiguration {
                                reason: format!(
                                    "{}: at least one of maxSizeBytes or rotationInterval must be \
                                     > 0",
                                    stream_name
                                ),
                            },
                        });
                    }
                }
            }
        }

        // 2.5: Probe validation
        for (_probe_name, probe) in
            [("readinessProbe", &config.readiness_probe), ("livenessProbe", &config.liveness_probe)]
        {
            if let Some(p) = probe {
                // timeout <= period
                let timeout_secs = p.timeout.as_secs();
                let period_secs = p.period.as_secs();
                if timeout_secs > period_secs {
                    return Err(Error::Validate {
                        source: ValidationError::InvalidProbeTimeout {
                            timeout: timeout_secs,
                            period: period_secs,
                        },
                    });
                }
                // Port range validation for HTTP and TCP handlers
                match &p.handler {
                    ProbeHandlerConfig::HttpGet { port, .. }
                    | ProbeHandlerConfig::TcpSocket { port, .. } => {
                        if !(1..=65535).contains(port) {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidProbePort { port: *port },
                            });
                        }
                    }
                }
            }
        }

        // 2.6: Restart backoff validation
        if let Some(restart_policy) = &config.restart_policy {
            match restart_policy {
                RestartPolicyConfig::Always { backoff }
                | RestartPolicyConfig::OnFailure { backoff, .. } => {
                    if let Some(backoff) = backoff {
                        if backoff.is_zero() {
                            return Err(Error::Validate {
                                source: ValidationError::InvalidRestartBackoff {
                                    backoff: backoff.as_secs(),
                                },
                            });
                        }
                    }
                }
                RestartPolicyConfig::Never => {}
            }
        }

        // 2.7: Detect duplicate environment variables
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();
        for (key, _) in &config.environment_variables {
            if !seen.insert(key) {
                duplicates.push(key.clone());
            }
        }
        if !duplicates.is_empty() {
            return Err(Error::Validate {
                source: ValidationError::DuplicateEnvironmentVariables {
                    process: name.to_string(),
                    variables: duplicates,
                },
            });
        }

        // 2.8: Warn if rotation destination is Null/Inherit with rotation configured
        if let Some(log) = &config.log {
            for (stream_name, stream_config) in [("stdout", &log.stdout), ("stderr", &log.stderr)] {
                if let Some(_rotation) = &stream_config.rotation {
                    match stream_config.destination {
                        LogDestination::Null | LogDestination::Inherit => {
                            eprintln!(
                                "Warning: Process '{}' has rotation configured for {} stream but \
                                 destination is {:?}; rotation will have no effect.",
                                name, stream_name, stream_config.destination
                            );
                        }
                        LogDestination::File { .. } => {}
                    }
                }
            }
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
    #[allow(clippy::option_if_let_else)]
    fn detect_dependency_cycles(&self) -> Result<(), Error> {
        let mut graph = DiGraph::<String, ()>::new();
        let mut indices = HashMap::new();

        for name in self.processes.keys() {
            let _ = indices.insert(name.clone(), graph.add_node(name.clone()));
        }

        for (name, config) in &self.processes {
            let from = indices[&name.clone()];
            for dep_name in config.depends_on.keys() {
                let to = indices[&dep_name.clone()];
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
            if let Some(cycle_nodes) = Self::find_cycle_in_scc(&graph, &scc, node) {
                let cycle = cycle_nodes.into_iter().map(|idx| graph[idx].clone()).collect();
                Err(Error::Validate { source: ValidationError::CyclicDependency { cycle } })
            } else {
                // Could not find a cycle? Return single node as fallback
                Err(Error::Validate {
                    source: ValidationError::CyclicDependency { cycle: vec![node_name] },
                })
            }
        } else {
            Ok(())
        }
    }

    /// Find a cycle within the given strongly connected component starting from
    /// `start`. Returns a list of node indices representing the cycle,
    /// where the first and last nodes are the same (the cycle is closed).
    #[allow(unused_results, clippy::collection_is_never_read)]
    fn find_cycle_in_scc(
        graph: &DiGraph<String, ()>,
        scc: &[petgraph::graph::NodeIndex],
        start: petgraph::graph::NodeIndex,
    ) -> Option<Vec<petgraph::graph::NodeIndex>> {
        let scc_set: HashSet<_> = scc.iter().copied().collect();
        let mut stack = Vec::new();
        let mut on_stack = HashSet::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<petgraph::graph::NodeIndex, petgraph::graph::NodeIndex> =
            HashMap::new();

        stack.push(start);
        on_stack.insert(start);
        visited.insert(start);

        while let Some(&node) = stack.last() {
            // Explore neighbors within the SCC
            let mut found_next = false;
            for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
                if !scc_set.contains(&neighbor) {
                    continue;
                }
                if visited.insert(neighbor) {
                    parent.insert(neighbor, node);
                    stack.push(neighbor);
                    on_stack.insert(neighbor);
                    found_next = true;
                    break;
                } else if on_stack.contains(&neighbor) {
                    // Back edge found: node -> neighbor, and neighbor is on the current DFS stack.
                    // Cycle: neighbor -> ... -> node -> neighbor.
                    // Collect nodes from stack from neighbor to node.
                    let mut cycle = Vec::new();
                    for &idx in stack.iter().rev() {
                        cycle.push(idx);
                        if idx == neighbor {
                            break;
                        }
                    }
                    cycle.reverse(); // now from neighbor to node
                    cycle.push(neighbor); // close the cycle
                    return Some(cycle);
                }
            }
            if !found_next {
                stack.pop();
                on_stack.remove(&node);
            }
        }
        None
    }
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
                let convert = |s: LogStreamConfig| -> SupLogStreamConfig {
                    let dest = match s.destination {
                        LogDestination::Null => SupLogDestination::Null,
                        LogDestination::Inherit => SupLogDestination::Inherit,
                        LogDestination::File { path } => SupLogDestination::File { path },
                    };
                    let rotation = s.rotation.map(|r| SupLogRotationConfig {
                        max_size_bytes: r.max_size_bytes.map(|s| s.as_u64()),
                        rotation_interval_secs: r.rotation_interval.map(|d| d.as_secs()),
                        max_files: r.max_files,
                        max_age_days: r.max_age_days,
                        mode: r.mode.and_then(|m| u32::from_str_radix(&m, 8).ok()),
                        compression: r.compression.map(|c| match c {
                            LogCompression::Gzip => SupLogCompression::Gzip,
                        }),
                    });
                    SupLogStreamConfig { destination: dest, rotation }
                };
                (convert(stdout), convert(stderr))
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
