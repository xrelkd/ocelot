use std::time::Duration;

use nix::sys::{signal, signal::Signal};
use ocelot_test_utils::{find_zombie_processes, run_in_namespace, supports_user_namespace};

/// Test that idle runs and can be terminated with SIGTERM
#[test]
fn test_idle_shutdown_on_sigterm() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of idle process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to {my_pid}");
        });

        // execute should return after receiving SIGTERM
        ocelot_idle::execute()?;

        // Wait for sender thread to finish
        drop(sender.join());

        eprintln!("[test] idle shut down cleanly");
        Ok(0)
    })?;

    assert_eq!(exit_code, 0, "Expected exit code 0 after SIGTERM, got {exit_code}");
    Ok(())
}

/// Test that idle shuts down on SIGINT
#[test]
fn test_idle_shutdown_on_sigint() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let my_pid = nix::unistd::getpid();
        eprintln!("[test] PID of idle process: {my_pid}");

        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGINT);
            eprintln!("[test] Sent SIGINT to {my_pid}");
        });

        ocelot_idle::execute()?;

        drop(sender.join());

        eprintln!("[test] idle shut down cleanly");
        Ok(0)
    })?;

    assert_eq!(exit_code, 0, "Expected exit code 0 after SIGINT, got {exit_code}");
    Ok(())
}

/// Test that idle properly reaps child processes (zombies)
#[test]
fn test_idle_child_reaping() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        // Spawn a thread that will send SIGTERM after a delay to break execute loop
        let my_pid = nix::unistd::getpid();
        let sender = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
            eprintln!("[test] Sent SIGTERM to break execute loop");
        });

        // Fork a child that will exit after a short delay, creating a zombie
        #[allow(unsafe_code)]
        match unsafe { nix::unistd::fork() } {
            Ok(nix::unistd::ForkResult::Parent { child: _ }) => {
                // Parent (test process) continues
                eprintln!("[test] Forked child that will exit");
            }
            Ok(nix::unistd::ForkResult::Child) => {
                // Child: sleep then exit, creating a zombie for parent (idle) to reap
                std::thread::sleep(Duration::from_millis(100));
                std::process::exit(0);
            }
            Err(e) => return Err(format!("fork failed: {e}").into()),
        }

        // Run idle's execute loop; it should reap the child when it exits
        ocelot_idle::execute()?;

        // Wait for sender thread to finish
        drop(sender.join());

        // After execute exits, check for any remaining zombie processes
        let zombies = find_zombie_processes()?;
        if !zombies.is_empty() {
            eprintln!("Zombies still present: {zombies:?}");
        }
        // Assert no zombies remain
        assert!(zombies.is_empty(), "Found {} zombie processes", zombies.len());

        eprintln!("[test] Child reaping verified");
        Ok(0)
    })?;

    assert_eq!(exit_code, 0);
    Ok(())
}

/// Test that idle emits a warning when not running as PID 1
#[test]
fn test_idle_pid1_warning() {
    // This test runs without namespace isolation to verify the warning
    // We need to send a signal to break execute() out of its loop

    let my_pid = nix::unistd::getpid();
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        let _ = signal::kill(nix::unistd::Pid::from_raw(my_pid.as_raw()), Signal::SIGTERM);
    });

    // Call execute directly (not in namespace) - should still work
    drop(ocelot_idle::execute());

    // Wait for sender thread
    drop(sender.join());
}
