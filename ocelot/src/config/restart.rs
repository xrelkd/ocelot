use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "PascalCase", deny_unknown_fields)]
pub enum RestartPolicyConfig {
    #[default]
    Never,
    Always {
        #[serde(default, with = "humantime_serde")]
        backoff: Option<Duration>,
    },
    OnFailure {
        #[serde(default, rename = "maxRetries")]
        max_retries: Option<u32>,

        #[serde(default, with = "humantime_serde")]
        backoff: Option<Duration>,
    },
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::restart::RestartPolicyConfig;

    #[test]
    fn test_restart_policy_never_serde() {
        let yaml = r"
type: Never
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, RestartPolicyConfig::Never);
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
    fn test_restart_policy_always_without_backoff() {
        let yaml = r"
type: Always
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, RestartPolicyConfig::Always { backoff: None });
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
    fn test_restart_policy_on_failure_partial() {
        let yaml = r"
type: OnFailure
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, RestartPolicyConfig::OnFailure { max_retries: None, backoff: None });
    }

    #[test]
    fn test_restart_policy_default_is_never() {
        let config = RestartPolicyConfig::default();
        assert_eq!(config, RestartPolicyConfig::Never);
    }
}
