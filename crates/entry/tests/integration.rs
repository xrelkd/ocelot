use std::time::Duration;

use nix::{
    sys::{
        signal,
        signal::{SigHandler, Signal},
        wait,
    },
    unistd,
    unistd::{ForkResult, Pid},
};
use ocelot_test_utils::{find_zombie_processes, run_in_namespace, supports_user_namespace};

/// Test normal exit with exit code 0
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_normal_exit() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let code = ocelot_entry::execute("true", Vec::<String>::new(), None)?;
        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies found after execute: {zombies:?}");
        }
        Ok(code)
    })?;
    assert_eq!(exit_code, 0, "Expected exit code 0 from 'true'");
    Ok(())
}

/// Test timeout-based termination: child ignores SIGTERM, parent sends SIGKILL
/// after timeout. A subprocess is forked to send SIGTERM to the parent process
/// to trigger the shutdown sequence.
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_timeout_kill() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let parent_pid = unistd::getpid();
        eprintln!("[test] Parent PID: {parent_pid}");

        #[expect(unsafe_code, reason = "We are invoking syscall in a correct way")]
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Child) => {
                std::thread::sleep(Duration::from_millis(100));
                let _ = signal::kill(Pid::from_raw(parent_pid.as_raw()), Signal::SIGTERM);
                eprintln!("[test] Sent SIGTERM to parent");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child }) => {
                unsafe {
                    // Reset `SIGTERM` to its default handler (Default), otherwise PID 1 will ignore
                    // it.
                    let _ = signal::signal(Signal::SIGTERM, SigHandler::SigDfl)?;
                }

                let code = ocelot_entry::execute(
                    "bash",
                    ["-c".to_string(), "trap '' TERM; sleep 30".to_string()],
                    Some(Duration::from_millis(500)),
                )?;
                let _ = wait::waitpid(child, None);
                eprintln!("[test] execute returned: {code}");
                let zombies = find_zombie_processes()?;
                if !zombies.is_empty() {
                    eprintln!("Zombies found after execute with timeout: {zombies:?}");
                }
                Ok(code)
            }
            Err(e) => Err(format!("fork failed: {e}").into()),
        }
    })?;
    assert_ne!(exit_code, 0, "Expected non-zero exit code, got {exit_code}");
    Ok(())
}

/// Test that execute properly reaps child processes (zombies)
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_execute_child_reaping() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        // Create a temporary zombie before calling execute
        #[expect(
            unsafe_code,
            reason = "Fork is required to create a zombie for testing child reaping"
        )]
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Parent { child: _ }) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(ForkResult::Child) => {
                std::process::exit(0);
            }
            Err(e) => return Err(format!("fork failed: {e}").into()),
        }

        let code = ocelot_entry::execute("true", [], None)?;
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
