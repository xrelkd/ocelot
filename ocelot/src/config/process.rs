use std::{collections::HashMap, path::PathBuf, str::FromStr, time::Duration};

use bytesize::ByteSize;
use nix::sys::signal::Signal;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{MapAccess, Visitor},
    ser::SerializeMap,
};

use crate::config::{
    dependency::DependencyConfig, probe::ProbeConfig, restart::RestartPolicyConfig,
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
    pub compression: Option<LogCompression>,
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
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogCompression {
    Gzip,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytesize::ByteSize;
    use nix::sys::signal::Signal;

    use crate::config::process::{LogRotationConfig, ProcessConfig, ShutdownSignalConfig};

    #[test]
    fn test_shutdown_signal_sigterm_explicit() {
        let yaml = r"
type: sigterm
";
        let config: ShutdownSignalConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, ShutdownSignalConfig::Sigterm);
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
