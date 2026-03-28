use std::{collections::HashMap, path::PathBuf, time::Duration};

use nix::sys::signal::{self, Signal};
use ocelot_supervise::{
    DependencyRegistry, OrchestratorConfig, Phase, Reaper, RestartPolicy, Supervisor,
    SupervisorConfig, supervisor_config, supervisor_probe,
};
use ocelot_test_utils::{find_zombie_processes, run_in_namespace, supports_user_namespace};
use tokio_util::sync::CancellationToken;

/// Test normal exit with exit code 0
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_normal_exit() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        let supervisor_config = SupervisorConfig {
            name: "test".to_string(),
            program: PathBuf::from("true"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies found after execute: {zombies:?}");
        }
        Ok(code)
    })?;
    assert_eq!(exit_code, 0, "Expected exit code 0 from 'true'");
    Ok(())
}

/// Test timeout-based termination: send SIGTERM, verify child exits within
/// timeout
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_timeout_kill() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        let supervisor_config = SupervisorConfig {
            name: "sleep".to_string(),
            program: PathBuf::from("/usr/bin/env"),
            arguments: vec![
                "bash".to_string(),
                "-c".to_string(),
                "trap '' TERM; sleep 30".to_string(),
            ],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(1),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(3),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies found after execute with timeout: {zombies:?}");
        }
        Ok(code)
    })?;
    assert_eq!(exit_code, 0, "Expected exit code 0 after graceful shutdown");
    Ok(())
}

/// Test that execute properly reaps child processes (zombies)
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_child_reaping() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        #[allow(unsafe_code)]
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child: _ }) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(nix::unistd::ForkResult::Child) => {
                std::process::exit(0);
            }
            Err(e) => return Err(format!("fork failed: {e}").into()),
        }

        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        let supervisor_config = SupervisorConfig {
            name: "true".to_string(),
            program: PathBuf::from("true"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        std::thread::sleep(Duration::from_millis(100));

        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies still present: {zombies:?}");
        }
        Ok(code)
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test Supervisor basic lifecycle: start, monitor status, and shutdown
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_supervisor_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let _ = run_in_namespace(|| -> Result<i32, Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let (reaper, reaper_executor) = Reaper::new();

        let config = SupervisorConfig {
            name: "quick".to_string(),
            program: PathBuf::from("true"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let dependency_registry = DependencyRegistry::new(1024);
        let (supervisor, supervisor_executor) =
            Supervisor::new(config, reaper, dependency_registry);
        let cancel_token = CancellationToken::new();

        let _reaper_handle = rt.spawn(reaper_executor.serve(cancel_token.clone()));
        let _supervisor_handle = rt.spawn(supervisor_executor.run(cancel_token.clone()));

        let supervisor_clone = supervisor.clone();
        let _spawn_handle = rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            supervisor_clone.start();
        });

        rt.block_on(async { tokio::time::sleep(Duration::from_millis(200)).await });

        let status = rt.block_on(supervisor.get_status());
        eprintln!("Supervisor status: {status:?}");
        assert_eq!(status.phase, Phase::Completed);
        assert_eq!(status.last_exit_code, Some(0));
        assert_eq!(status.restart_count, 0);

        cancel_token.cancel();
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });

        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies found after supervisor lifecycle test: {zombies:?}");
        }
        assert!(zombies.is_empty());

        Ok(0)
    })?;
    Ok(())
}

/// Test restart policies: Never, Always, `OnFailure`
// RATIONALE: This function tests three restart policy scenarios (Never, Always, OnFailure)
// in a single test to share namespace setup/teardown code and avoid duplication.
// The 111 lines are justified due to the three independent test cases.
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
#[allow(clippy::too_many_lines)]
fn test_restart_policies() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    // Test Never policy - process exits and doesn't restart
    let _ = run_in_namespace(|| -> Result<i32, Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let (reaper, reaper_executor) = Reaper::new();

        let config = SupervisorConfig {
            name: "never_test".to_string(),
            program: PathBuf::from("false"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let dependency_registry = DependencyRegistry::new(1024);
        let (supervisor, supervisor_executor) =
            Supervisor::new(config, reaper, dependency_registry);
        let cancel_token = CancellationToken::new();

        let _reaper_handle = rt.spawn(reaper_executor.serve(cancel_token.clone()));
        let _supervisor_handle = rt.spawn(supervisor_executor.run(cancel_token.clone()));

        let supervisor_clone = supervisor.clone();
        let _spawn_handle = rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            supervisor_clone.start();
        });

        rt.block_on(async { tokio::time::sleep(Duration::from_millis(200)).await });

        let status = rt.block_on(supervisor.get_status());
        eprintln!("Never policy status: {status:?}");
        assert!(matches!(status.phase, Phase::CrashLoopBackOff));
        assert_eq!(status.restart_count, 0);

        cancel_token.cancel();
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });

        Ok(0)
    })?;

    // Test Always policy - always restarts regardless of exit code
    let _ = run_in_namespace(|| -> Result<i32, Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let (reaper, reaper_executor) = Reaper::new();

        let config = SupervisorConfig {
            name: "always_test".to_string(),
            program: PathBuf::from("false"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Always { backoff: Duration::from_millis(100) },
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let dependency_registry = DependencyRegistry::new(1024);
        let (supervisor, supervisor_executor) =
            Supervisor::new(config, reaper, dependency_registry);
        let cancel_token = CancellationToken::new();

        let _reaper_handle = rt.spawn(reaper_executor.serve(cancel_token.clone()));
        let _supervisor_handle = rt.spawn(supervisor_executor.run(cancel_token.clone()));

        let supervisor_clone = supervisor.clone();
        let _spawn_handle = rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            supervisor_clone.start();
        });

        rt.block_on(async { tokio::time::sleep(Duration::from_millis(1000)).await });

        let status = rt.block_on(supervisor.get_status());
        eprintln!("Always policy status after 1000ms: {status:?}");
        assert!(
            status.restart_count >= 1,
            "Expected at least 1 restart, got {}",
            status.restart_count
        );
        assert!(matches!(status.phase, Phase::Running | Phase::CrashLoopBackOff));

        cancel_token.cancel();
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });

        Ok(0)
    })?;

    // Test OnFailure policy - restarts only on non-zero exit
    let _ = run_in_namespace(|| -> Result<i32, Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let (reaper, reaper_executor) = Reaper::new();

        let config = SupervisorConfig {
            name: "onfailure_test".to_string(),
            program: PathBuf::from("false"),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::OnFailure {
                max_retries: 5,
                backoff: Duration::from_millis(100),
            },
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let dependency_registry = DependencyRegistry::new(1024);
        let (supervisor, supervisor_executor) =
            Supervisor::new(config, reaper, dependency_registry);
        let cancel_token = CancellationToken::new();

        let _reaper_handle = rt.spawn(reaper_executor.serve(cancel_token.clone()));
        let _supervisor_handle = rt.spawn(supervisor_executor.run(cancel_token.clone()));

        let supervisor_clone = supervisor.clone();
        let _spawn_handle = rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            supervisor_clone.start();
        });

        rt.block_on(async { tokio::time::sleep(Duration::from_millis(300)).await });

        let status = rt.block_on(supervisor.get_status());
        eprintln!("OnFailure policy status after 300ms: {status:?}");
        assert!(status.restart_count > 0, "Expected restarts on failure");
        assert!(matches!(status.phase, Phase::Running | Phase::CrashLoopBackOff));

        cancel_token.cancel();
        rt.block_on(async { tokio::time::sleep(Duration::from_millis(100)).await });

        Ok(0)
    })?;

    Ok(())
}

/// Test multiple supervisors running under Orchestrator
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_multiple_supervisors() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        let config1 = SupervisorConfig {
            name: "sleep1".to_string(),
            program: PathBuf::from("sleep"),
            arguments: vec!["300".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let config2 = SupervisorConfig {
            name: "sleep2".to_string(),
            program: PathBuf::from("sleep"),
            arguments: vec!["300".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![config1, config2],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        Ok(code)
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test process dependencies - a process depending on another
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_process_dependencies() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        // First process (dependency)
        let parent_config = SupervisorConfig {
            name: "parent".to_string(),
            program: PathBuf::from("sleep"),
            arguments: vec!["300".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        // Second process depends on parent
        let mut dep = HashMap::new();
        let _ = dep
            .insert("parent".to_string(), supervisor_config::ProcessDependency { condition: None });

        let child_config = SupervisorConfig {
            name: "child".to_string(),
            program: PathBuf::from("sleep"),
            arguments: vec!["300".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: dep,
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![parent_config, child_config],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        Ok(code)
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test signal forwarding from supervisor to child process
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_signal_forwarding() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        let supervisor_config = SupervisorConfig {
            name: "sleep".to_string(),
            program: PathBuf::from("sleep"),
            arguments: vec!["300".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        Ok(code)
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test readiness and liveness probes
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_probes() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of execute process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        // Use nc to listen on port 8080 for readiness probe
        let readiness_probe = supervisor_probe::Probe {
            handler: supervisor_probe::ProbeHandler::TcpSocket {
                host: Some("127.0.0.1".to_string()),
                port: 8080,
            },
            initial_delay: Duration::ZERO,
            period: Duration::from_millis(100),
            timeout: Duration::from_millis(50),
            failure_threshold: 3,
            success_threshold: 1,
        };

        let liveness_probe = supervisor_probe::Probe {
            handler: supervisor_probe::ProbeHandler::TcpSocket {
                host: Some("127.0.0.1".to_string()),
                port: 8080,
            },
            initial_delay: Duration::ZERO,
            period: Duration::from_millis(200),
            timeout: Duration::from_millis(50),
            failure_threshold: 3,
            success_threshold: 1,
        };

        let supervisor_config = SupervisorConfig {
            name: "netcat".to_string(),
            program: PathBuf::from("nc"),
            arguments: vec!["-l".to_string(), "-p".to_string(), "8080".to_string()],
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: Some(readiness_probe),
            liveness_probe: Some(liveness_probe),
            restart_policy: RestartPolicy::Never,
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        };

        let orchestrator_config = OrchestratorConfig {
            supervisors: vec![supervisor_config],
            shutdown_timeout: Duration::from_secs(30),
        };

        let code = ocelot_supervise::execute(orchestrator_config)?;

        drop(sender.join());

        eprintln!("[test] execute returned: {code}");
        Ok(code)
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test exit code propagation from child process
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_exit_code_propagation() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    // Test various exit codes
    for &expected_exit_code in &[0, 1, 42, 127, 255] {
        let exit_code = run_in_namespace(move || -> Result<i32, Box<dyn std::error::Error>> {
            let my_pid = nix::unistd::getpid();
            eprintln!("[test] PID: {my_pid}, expected exit: {expected_exit_code}");

            let sender = std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            });

            let supervisor_config = SupervisorConfig {
                name: format!("exit_code_{expected_exit_code}"),
                program: PathBuf::from("/usr/bin/env"),
                arguments: vec![
                    "bash".to_string(),
                    "-c".to_string(),
                    format!("exit {expected_exit_code}"),
                ],
                environment_variables: HashMap::new(),
                working_directory: None,
                depends_on: HashMap::new(),
                readiness_probe: None,
                liveness_probe: None,
                restart_policy: RestartPolicy::Never,
                shutdown_signal: None,
                termination_grace_period: Duration::from_secs(30),
            };

            let orchestrator_config = OrchestratorConfig {
                supervisors: vec![supervisor_config],
                shutdown_timeout: Duration::from_secs(30),
            };

            let code = ocelot_supervise::execute(orchestrator_config)?;

            drop(sender.join());
            Ok(code)
        })?;
        assert_eq!(exit_code, 0, "Orchestrator should return 0 on graceful shutdown");
    }

    Ok(())
}
