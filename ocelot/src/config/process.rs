use std::{collections::HashMap, str::FromStr};

use nix::sys::signal::Signal;
use serde::{Deserialize, Serialize};

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

    #[serde(default)]
    pub environment_variables: HashMap<String, String>,

    pub working_directory: Option<String>,

    #[serde(default)]
    pub depends_on: HashMap<String, DependencyConfig>,

    pub readiness_probe: Option<ProbeConfig>,

    pub liveness_probe: Option<ProbeConfig>,

    pub restart_policy: Option<RestartPolicyConfig>,

    #[serde(default)]
    pub shutdown_signal: Option<ShutdownSignalConfig>,

    #[serde(default = "default_termination_grace_period_secs")]
    pub termination_grace_period_secs: u64,
}

const fn default_termination_grace_period_secs() -> u64 { 60 }

#[cfg(test)]
mod tests {
    use nix::sys::signal::Signal;

    use super::{ProcessConfig, ShutdownSignalConfig};

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
        assert_eq!(config.termination_grace_period_secs, 60);
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
terminationGracePeriodSecs: 30
";
        let config: ProcessConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.program, "/usr/bin/myapp");
        assert_eq!(config.arguments, vec!["--config", "/etc/config.yaml"]);
        assert_eq!(config.environment_variables.get("LOG_LEVEL"), Some(&"debug".to_string()));
        assert_eq!(config.working_directory, Some("/app".to_string()));
        assert_eq!(config.termination_grace_period_secs, 30);
    }
}
