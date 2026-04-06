use std::{collections::HashMap, path::PathBuf, str::FromStr, time::Duration};

use bytesize::ByteSize;
use error::Error;
use nix::sys::signal::Signal;
use ocelot_supervise::{
    LogCompression as SupLogCompression, LogDestination as SupLogDestination,
    LogRotationConfig as SupLogRotationConfig, LogStreamConfig as SupLogStreamConfig,
    supervisor_config, supervisor_probe,
};
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeMap,
};

use crate::config::{
    error,
    supervise::{
        dependency::DependencyConfig,
        probe::{ProbeConfig, ProbeHandlerConfig},
        restart::RestartPolicyConfig,
    },
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase", deny_unknown_fields)]
pub enum ShutdownSignalConfig {
    #[default]
    #[serde(rename = "sigterm")]
    Sigterm,
    #[serde(rename = "number")]
    Number(u8),
    #[serde(rename = "name")]
    Name(String),
}

impl ShutdownSignalConfig {
    pub fn to_signal(&self) -> Signal {
        match self {
            Self::Sigterm => Signal::SIGTERM,
            Self::Number(n) => Signal::try_from(i32::from(*n)).unwrap_or(Signal::SIGTERM),
            Self::Name(name) => Signal::from_str(name).unwrap_or(Signal::SIGTERM),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProcessConfig {
    pub program: String,

    #[serde(default)]
    pub arguments: Vec<String>,

    #[serde(
        default,
        deserialize_with = "deserialize_env_vars",
        serialize_with = "serialize_env_vars"
    )]
    pub environment_variables: Vec<(String, String)>,

    pub working_directory: Option<String>,

    #[serde(default)]
    pub depends_on: HashMap<String, DependencyConfig>,

    pub readiness_probe: Option<ProbeConfig>,

    pub liveness_probe: Option<ProbeConfig>,

    pub restart_policy: Option<RestartPolicyConfig>,

    #[serde(default)]
    pub shutdown_signal: Option<ShutdownSignalConfig>,

    #[serde(with = "humantime_serde", default = "default_termination_grace_period")]
    pub termination_grace_period: Duration,

    #[serde(default)]
    pub log: Option<LogConfig>,
}

const fn default_termination_grace_period() -> Duration { Duration::from_secs(60) }

impl ProcessConfig {
    /// Returns the value of the given environment variable key, if present.
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.environment_variables.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// Validates this process configuration.
    pub fn validate(&self, name: &str) -> Result<(), Error> {
        self.validate_program(name)?;
        self.validate_termination_grace_period()?;
        self.validate_log_rotation(name)?;
        self.validate_probes()?;
        self.validate_restart_backoff()?;
        self.validate_environment_variables(name)?;
        Ok(())
    }

    fn validate_program(&self, name: &str) -> Result<(), Error> {
        snafu::ensure!(
            !self.program.is_empty(),
            error::MissingProcessProgramSnafu { process: name.to_string() }
        );
        Ok(())
    }

    fn validate_termination_grace_period(&self) -> Result<(), Error> {
        snafu::ensure!(
            !self.termination_grace_period.is_zero(),
            error::InvalidTerminationGracePeriodSnafu {
                value: self.termination_grace_period.as_secs()
            }
        );
        Ok(())
    }

    fn validate_log_rotation(&self, name: &str) -> Result<(), Error> {
        let Some(log) = &self.log else { return Ok(()) };

        for (stream_name, stream_config) in [("stdout", &log.stdout), ("stderr", &log.stderr)] {
            if let Some(rotation) = &stream_config.rotation {
                let checks = [
                    (
                        rotation.max_size_bytes.map(|s| s.as_u64()),
                        format!("{stream_name}.maxSizeBytes"),
                    ),
                    (
                        rotation.rotation_interval.map(|d| d.as_secs()),
                        format!("{stream_name}.rotationInterval"),
                    ),
                    (rotation.max_files.map(u64::from), format!("{stream_name}.maxFiles")),
                    (rotation.max_age_days.map(u64::from), format!("{stream_name}.maxAgeDays")),
                ];

                for (value, field) in checks {
                    snafu::ensure!(
                        value != Some(0),
                        error::InvalidLogRotationSnafu { field, value: 0 }
                    );
                }

                let has_size = rotation.max_size_bytes.is_some_and(|s| s.as_u64() > 0);
                let has_interval = rotation.rotation_interval.is_some_and(|d| d.as_secs() > 0);
                snafu::ensure!(
                    has_size || has_interval,
                    error::InvalidRotationConfigurationSnafu {
                        reason: format!(
                            "{stream_name}: at least one of maxSizeBytes or rotationInterval must \
                             be > 0"
                        )
                    }
                );

                match stream_config.destination {
                    LogDestination::Null | LogDestination::Inherit => {
                        eprintln!(
                            "Warning: Process '{name}' has rotation configured for {stream_name} \
                             stream but destination is {:?}; rotation will have no effect.",
                            stream_config.destination
                        );
                    }
                    LogDestination::File { .. } => {}
                }
            }
        }
        Ok(())
    }

    fn validate_probes(&self) -> Result<(), Error> {
        for probe in [&self.readiness_probe, &self.liveness_probe] {
            let Some(p) = probe else { continue };

            let timeout_secs = p.timeout.as_secs();
            let period_secs = p.period.as_secs();
            snafu::ensure!(
                timeout_secs <= period_secs,
                error::InvalidProbeTimeoutSnafu { timeout: timeout_secs, period: period_secs }
            );

            match &p.handler {
                ProbeHandlerConfig::HttpGet { port, .. }
                | ProbeHandlerConfig::TcpSocket { port, .. } => {
                    snafu::ensure!(
                        (1..=65535).contains(port),
                        error::InvalidProbePortSnafu { port: *port }
                    );
                }
            }
        }
        Ok(())
    }

    fn validate_restart_backoff(&self) -> Result<(), Error> {
        let Some(restart_policy) = &self.restart_policy else { return Ok(()) };

        match restart_policy {
            RestartPolicyConfig::Always { backoff }
            | RestartPolicyConfig::OnFailure { backoff, .. } => {
                if let Some(backoff) = backoff {
                    snafu::ensure!(
                        !backoff.is_zero(),
                        error::InvalidRestartBackoffSnafu { backoff: backoff.as_secs() }
                    );
                }
            }
            RestartPolicyConfig::Never => {}
        }
        Ok(())
    }

    fn validate_environment_variables(&self, name: &str) -> Result<(), Error> {
        let (_seen, duplicates): (_, Vec<_>) = self.environment_variables.iter().fold(
            (std::collections::HashSet::new(), Vec::new()),
            |(mut seen, mut dups), (key, _)| {
                if !seen.insert(key) {
                    dups.push(key.clone());
                }
                (seen, dups)
            },
        );
        snafu::ensure!(
            duplicates.is_empty(),
            error::DuplicateEnvironmentVariablesSnafu {
                process: name.to_string(),
                variables: duplicates
            }
        );
        Ok(())
    }
}

// Custom deserializer/serializer for environment_variables as Vec<(String,
// String)> to preserve order and allow duplicate detection.
fn deserialize_env_vars<'de, D>(deserializer: D) -> Result<Vec<(String, String)>, D::Error>
where
    D: Deserializer<'de>,
{
    struct VecVisitor;
    impl<'de> Visitor<'de> for VecVisitor {
        type Value = Vec<(String, String)>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a mapping of environment variables")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some((key, value)) = map.next_entry()? {
                vec.push((key, value));
            }
            Ok(vec)
        }
    }
    deserializer.deserialize_map(VecVisitor)
}

fn serialize_env_vars<S>(vec: &Vec<(String, String)>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(vec.len()))?;
    for (k, v) in vec {
        map.serialize_entry(k, v)?;
    }
    map.end()
}

/// Top-level logging configuration for a process.
///
/// Contains separate configurations for stdout and stderr streams.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogConfig {
    /// Configuration for standard output.
    #[serde(default)]
    pub stdout: LogStreamConfig,

    /// Configuration for standard error.
    #[serde(default)]
    pub stderr: LogStreamConfig,
}

// Log configuration types
/// Destination for log output.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum LogDestination {
    Null,
    Inherit,
    File { path: PathBuf },
}

/// Configuration for log file rotation.
///
/// Rotation can be triggered based on maximum file size, time interval, or
/// both. If both are specified, whichever condition is met first will trigger
/// rotation.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogRotationConfig {
    /// Maximum size in bytes before rotating. Accepts raw integers or
    /// human-readable strings like "10MB", "1GB". None means no size limit.
    #[serde(default)]
    pub max_size_bytes: Option<ByteSize>,
    /// Time interval for rotation as a human-readable duration (e.g., "1h",
    /// "24h"). None means no time-based rotation.
    #[serde(default, with = "humantime_serde")]
    pub rotation_interval: Option<Duration>,
    /// Maximum number of rotated files to retain. Older files are deleted.
    pub max_files: Option<u32>,
    /// Maximum age in days before auto-deleting rotated files.
    pub max_age_days: Option<u32>,
    /// File creation mode (permissions) for log files as an octal string
    /// (e.g., "644", "600").
    #[serde(default)]
    pub mode: Option<String>,
    /// Compression algorithm for rotated log files.
    #[serde(default)]
    pub compression: LogCompression,
}

/// Configuration for a single log stream (stdout or stderr).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogStreamConfig {
    /// Where to send the log output.
    pub destination: LogDestination,
    /// Optional rotation configuration, only applicable when destination is
    /// `File`.
    #[serde(default)]
    pub rotation: Option<LogRotationConfig>,
}

impl Default for LogStreamConfig {
    fn default() -> Self { Self { destination: LogDestination::Inherit, rotation: None } }
}

/// Compression algorithm for rotated log files.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogCompression {
    #[default]
    None,
    Gzip,
    Lz4,
}

impl From<ProcessConfig> for ocelot_supervise::SupervisorConfig {
    fn from(
        ProcessConfig {
            program,
            arguments,
            environment_variables,
            working_directory,
            depends_on,
            readiness_probe,
            liveness_probe,
            restart_policy,
            shutdown_signal,
            termination_grace_period,
            log,
        }: ProcessConfig,
    ) -> Self {
        let depends_on = depends_on
            .into_iter()
            .map(|(name, dep)| {
                let condition = dep.condition.map(supervisor_config::DependencyCondition::from);
                (name, supervisor_config::ProcessDependency { condition })
            })
            .collect();

        let environment_variables = environment_variables.into_iter().collect::<HashMap<_, _>>();

        let (log_stdout, log_stderr) = match log {
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
            program: PathBuf::from(program),
            arguments,
            environment_variables,
            working_directory: working_directory.map(PathBuf::from),
            depends_on,
            readiness_probe: readiness_probe.map(supervisor_probe::Probe::from),
            liveness_probe: liveness_probe.map(supervisor_probe::Probe::from),
            restart_policy: supervisor_config::RestartPolicy::from(
                restart_policy.unwrap_or_default(),
            ),
            shutdown_signal: shutdown_signal.map(|s| s.to_signal()),
            termination_grace_period,
            log_stdout,
            log_stderr,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytesize::ByteSize;
    use nix::sys::signal::Signal;

    use super::{LogRotationConfig, ProcessConfig, ShutdownSignalConfig};

    #[test]
    fn test_shutdown_signal_sigterm_explicit() {
        let yaml = r"
type: sigterm
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Sigterm);
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_shutdown_signal_number() {
        let yaml = r"
type: number
value: 9
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Number(9));
        assert_eq!(config.to_signal(), Signal::SIGKILL);
    }

    #[test]
    fn test_shutdown_signal_number_sigint() {
        let yaml = r"
type: number
value: 2
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Number(2));
        assert_eq!(config.to_signal(), Signal::SIGINT);
    }

    #[test]
    fn test_shutdown_signal_name_sigterm() {
        let yaml = r"
type: name
value: SIGTERM
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Name("SIGTERM".to_string()));
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_shutdown_signal_name_lowercase_full() {
        let yaml = r"
type: name
value: SIGHUP
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.to_signal(), Signal::SIGHUP);
    }

    #[test]
    fn test_shutdown_signal_name_invalid_fallback() {
        let yaml = r"
type: name
value: INVALID_SIGNAL
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.to_signal(), Signal::SIGTERM);
    }

    #[test]
    fn test_process_config_minimal() {
        let yaml = r"
program: /usr/bin/myapp
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.program, "/usr/bin/myapp");
        assert!(config.arguments.is_empty());
        assert_eq!(config.termination_grace_period, Duration::from_secs(60));
    }

    #[test]
    fn test_process_config_full() {
        let yaml = r"
program: /usr/bin/myapp
arguments:
  - --config
  - /etc/config.yaml
environmentVariables:
  LOG_LEVEL: debug
workingDirectory: /app
terminationGracePeriod: 30s
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.program, "/usr/bin/myapp");
        assert_eq!(config.arguments, vec!["--config", "/etc/config.yaml"]);
        assert_eq!(config.get_env("LOG_LEVEL"), Some(&"debug".to_string()));
        assert_eq!(config.working_directory, Some("/app".to_string()));
        assert_eq!(config.termination_grace_period, Duration::from_secs(30));
    }

    #[test]
    fn test_log_rotation_config_duration() {
        let yaml = r"
maxSizeBytes: 10485760
rotationInterval: 24h
maxFiles: 7
";
        let config: LogRotationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.max_size_bytes, Some(ByteSize::b(10_485_760)));
        assert_eq!(config.rotation_interval, Some(Duration::from_secs(86400)));
        assert_eq!(config.max_files, Some(7));
    }

    #[test]
    fn test_size_human_readable_mb() {
        let yaml = r"
maxSizeBytes: 10MB
rotationInterval: 24h
maxFiles: 7
";
        let config: LogRotationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.max_size_bytes, Some(ByteSize::mb(10)));
    }

    #[test]
    fn test_size_human_readable_gb() {
        let yaml = r"
maxSizeBytes: 1GB
";
        let config: LogRotationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.max_size_bytes, Some(ByteSize::gb(1)));
    }

    #[test]
    fn test_size_human_readable_kb() {
        let yaml = r"
maxSizeBytes: 512KB
";
        let config: LogRotationConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.max_size_bytes, Some(ByteSize::kb(512)));
    }

    #[test]
    fn test_size_invalid_format() {
        let yaml = r"
maxSizeBytes: not_a_size
";
        let result: Result<LogRotationConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err());
    }
}
