use std::time::Duration;

use ocelot_supervise::supervisor_probe;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeConfig {
    #[serde(default)]
    pub handler: ProbeHandlerConfig,

    #[serde(with = "humantime_serde", default = "default_initial_delay")]
    pub initial_delay: Duration,

    #[serde(with = "humantime_serde", default = "default_period")]
    pub period: Duration,

    #[serde(with = "humantime_serde", default = "default_timeout")]
    pub timeout: Duration,

    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: i32,

    #[serde(default = "default_success_threshold")]
    pub success_threshold: i32,
}

const fn default_period() -> Duration { Duration::from_secs(10) }
const fn default_timeout() -> Duration { Duration::from_secs(1) }
const fn default_initial_delay() -> Duration { Duration::from_secs(0) }
const fn default_failure_threshold() -> i32 { 3 }
const fn default_success_threshold() -> i32 { 1 }

impl Default for ProbeHandlerConfig {
    fn default() -> Self { Self::HttpGet { host: None, path: "/".to_string(), port: 80 } }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProbeHandlerConfig {
    HttpGet { host: Option<String>, path: String, port: u16 },
    TcpSocket { host: Option<String>, port: u16 },
}

impl From<ProbeConfig> for supervisor_probe::Probe {
    fn from(
        ProbeConfig {
            handler,
            initial_delay,
            period,
            timeout,
            failure_threshold,
            success_threshold,
        }: ProbeConfig,
    ) -> Self {
        let handler = match handler {
            ProbeHandlerConfig::HttpGet { host, path, port } => {
                supervisor_probe::ProbeHandler::HttpGet { host, path, port }
            }
            ProbeHandlerConfig::TcpSocket { host, port } => {
                supervisor_probe::ProbeHandler::TcpSocket { host, port }
            }
        };

        Self { handler, initial_delay, period, timeout, failure_threshold, success_threshold }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{ProbeConfig, ProbeHandlerConfig};

    #[test]
    fn test_probe_handler_http_get_serde() {
        let yaml = r"
type: httpGet
path: /health
port: 8080
";
        let handler: ProbeHandlerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            handler,
            ProbeHandlerConfig::HttpGet { host: None, path: "/health".to_string(), port: 8080 }
        );
    }

    #[test]
    fn test_probe_handler_http_get_with_host() {
        let yaml = r"
type: httpGet
host: localhost
path: /health
port: 8080
";
        let handler: ProbeHandlerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            handler,
            ProbeHandlerConfig::HttpGet {
                host: Some("localhost".to_string()),
                path: "/health".to_string(),
                port: 8080
            }
        );
    }

    #[test]
    fn test_probe_handler_tcp_socket_serde() {
        let yaml = r"
type: tcpSocket
port: 5432
";
        let handler: ProbeHandlerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(handler, ProbeHandlerConfig::TcpSocket { host: None, port: 5432 });
    }

    #[test]
    fn test_probe_handler_tcp_socket_with_host() {
        let yaml = r"
type: tcpSocket
host: 127.0.0.1
port: 5432
";
        let handler: ProbeHandlerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            handler,
            ProbeHandlerConfig::TcpSocket { host: Some("127.0.0.1".to_string()), port: 5432 }
        );
    }

    #[test]
    fn test_probe_handler_default() {
        let handler = ProbeHandlerConfig::default();
        assert_eq!(
            handler,
            ProbeHandlerConfig::HttpGet { host: None, path: "/".to_string(), port: 80 }
        );
    }

    #[test]
    fn test_probe_config_full() {
        let yaml = r"
handler:
  type: httpGet
  path: /ready
  port: 8080
initialDelay: 5s
period: 10s
timeout: 2s
failureThreshold: 3
successThreshold: 1
";
        let config: ProbeConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.initial_delay, Duration::from_secs(5));
        assert_eq!(config.period, Duration::from_secs(10));
        assert_eq!(config.timeout, Duration::from_secs(2));
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
    }

    #[test]
    fn test_probe_config_defaults() {
        let yaml = "{}";
        let config: ProbeConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.initial_delay, Duration::from_secs(0));
        assert_eq!(config.period, Duration::from_secs(10));
        assert_eq!(config.timeout, Duration::from_secs(1));
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.success_threshold, 1);
    }
}
