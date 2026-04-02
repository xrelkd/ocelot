use std::time::Duration;

use nix::sys::signal::Signal;
use ocelot_supervise::{LogDestination, supervisor_config::DependencyCondition};

use crate::config::{
    Error, ProcessConfig, SuperviseConfig,
    dependency::DependencyConfig,
    error::ValidationError,
    probe::{ProbeConfig, ProbeHandlerConfig},
    process::ShutdownSignalConfig,
    restart::RestartPolicyConfig,
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(config.shutdown_timeout_secs, 120);
    assert_eq!(config.processes.len(), 1);
    assert!(config.processes.contains_key("app"));
}

#[test]
fn test_template_basic_is_serializable() {
    let yaml_bytes = SuperviseConfig::template_basic();
    let yaml_str = String::from_utf8(yaml_bytes).expect("template is valid UTF-8");
    let config: SuperviseConfig = serde_yaml::from_str(&yaml_str).expect("template is valid YAML");
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate().is_ok());
}

#[test]
fn test_to_supervisors_empty() {
    let yaml = r#"
        version: "1.0"
        processes: {}
        "#;
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
    let err = config.validate().unwrap_err();
    match err {
        Error::Validate { source } => {
            assert!(matches!(source, ValidationError::DuplicateEnvironmentVariables { .. }));
            if let ValidationError::DuplicateEnvironmentVariables { process, variables } = source {
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
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
    let config: SuperviseConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.validate().is_ok());
}
