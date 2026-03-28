use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "PascalCase", deny_unknown_fields)]
pub enum RestartPolicyConfig {
    #[default]
    Never,
    Always {
        #[serde(default, rename = "backoffSecs")]
        backoff_secs: Option<u64>,
    },
    OnFailure {
        #[serde(default, rename = "maxRetries")]
        max_retries: Option<u32>,

        #[serde(default, rename = "backoffSecs")]
        backoff_secs: Option<u64>,
    },
}

#[cfg(test)]
mod tests {
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
backoffSecs: 5
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, RestartPolicyConfig::Always { backoff_secs: Some(5) });
    }

    #[test]
    fn test_restart_policy_always_without_backoff() {
        let yaml = r"
type: Always
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config, RestartPolicyConfig::Always { backoff_secs: None });
    }

    #[test]
    fn test_restart_policy_on_failure_full() {
        let yaml = r"
type: OnFailure
maxRetries: 10
backoffSecs: 3
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config,
            RestartPolicyConfig::OnFailure { max_retries: Some(10), backoff_secs: Some(3) }
        );
    }

    #[test]
    fn test_restart_policy_on_failure_partial() {
        let yaml = r"
type: OnFailure
";
        let config: RestartPolicyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            config,
            RestartPolicyConfig::OnFailure { max_retries: None, backoff_secs: None }
        );
    }

    #[test]
    fn test_restart_policy_default_is_never() {
        let config = RestartPolicyConfig::default();
        assert_eq!(config, RestartPolicyConfig::Never);
    }
}
