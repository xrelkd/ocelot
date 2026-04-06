use ocelot_supervise::supervisor_config;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyConfig {
    #[serde(default)]
    pub condition: Option<DependencyCondition>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum DependencyCondition {
    #[default]
    Started,
    Healthy,
    Completed,
    CompletedSuccessfully,
    LogReady,
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

#[cfg(test)]
mod tests {
    use super::{DependencyCondition, DependencyConfig};

    #[test]
    fn test_dependency_condition_started() {
        let yaml = "Started";
        let condition: DependencyCondition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(condition, DependencyCondition::Started);
    }

    #[test]
    fn test_dependency_condition_healthy() {
        let yaml = "Healthy";
        let condition: DependencyCondition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(condition, DependencyCondition::Healthy);
    }

    #[test]
    fn test_dependency_condition_completed() {
        let yaml = "Completed";
        let condition: DependencyCondition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(condition, DependencyCondition::Completed);
    }

    #[test]
    fn test_dependency_condition_completed_successfully() {
        let yaml = "CompletedSuccessfully";
        let condition: DependencyCondition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(condition, DependencyCondition::CompletedSuccessfully);
    }

    #[test]
    fn test_dependency_condition_log_ready() {
        let yaml = "LogReady";
        let condition: DependencyCondition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(condition, DependencyCondition::LogReady);
    }

    #[test]
    fn test_dependency_condition_default() {
        let condition = DependencyCondition::default();
        assert_eq!(condition, DependencyCondition::Started);
    }

    #[test]
    fn test_dependency_config_with_condition() {
        let yaml = r"
condition: Healthy
";
        let config: DependencyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.condition, Some(DependencyCondition::Healthy));
    }

    #[test]
    fn test_dependency_config_without_condition() {
        let yaml = "{}";
        let config: DependencyConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.condition, None);
    }
}
