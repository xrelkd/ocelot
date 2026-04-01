use std::time::Duration;

use nix::{
    sys::signal::{self, Signal},
    unistd::{self, ForkResult},
};
use ocelot_test_utils::{find_zombie_processes, run_in_namespace, supports_user_namespace};

/// Test that idle runs and can be terminated with SIGTERM
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_idle_shutdown_on_sigterm() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let parent_pid = unistd::getpid();
        eprintln!("[test] PID of idle process: {parent_pid}");

        #[expect(unsafe_code, reason = "Fork is required to create a child to send SIGTERM")]
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Child) => {
                std::thread::sleep(Duration::from_millis(100));
                let _ = signal::kill(parent_pid, Signal::SIGTERM);
                eprintln!("[test] Sent SIGTERM to parent");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child: _ }) => {
                // execute should return after receiving SIGTERM
                ocelot_idle::execute()?;
                eprintln!("[test] idle shut down cleanly");
                Ok(0)
            }
            Err(e) => Err(format!("fork failed: {e}").into()),
        }
    })?;

    assert_eq!(exit_code, 0, "Expected exit code 0 after SIGTERM, got {exit_code}");
    Ok(())
}

/// Test that idle shuts down on SIGINT
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_idle_shutdown_on_sigint() -> Result<(), Box<dyn std::error::Error>> {
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let parent_pid = unistd::getpid();
        eprintln!("[test] PID of idle process: {parent_pid}");

        #[expect(unsafe_code, reason = "Fork is required to create a child to send SIGINT")]
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Child) => {
                std::thread::sleep(Duration::from_millis(100));
                let _ = signal::kill(parent_pid, Signal::SIGINT);
                eprintln!("[test] Sent SIGINT to parent");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child: _ }) => {
                // execute should return after receiving SIGTERM
                ocelot_idle::execute()?;
                eprintln!("[test] idle shut down cleanly");
                Ok(0)
            }
            Err(e) => Err(format!("fork failed: {e}").into()),
        }
    })?;

    assert_eq!(exit_code, 0, "Expected exit code 0 after SIGINT, got {exit_code}");
    Ok(())
}

/// Test that idle properly reaps child processes (zombies)
#[test]
#[ignore = "requires user namespaces (unshare CLONE_NEWUSER) and root/CAP_SYS_ADMIN"]
fn test_idle_child_reaping() -> Result<(), Box<dyn std::error::Error>> {
    const ZOMBIE_COUNT: i32 = 10;
    assert!(supports_user_namespace(), "user namespaces not supported");

    let exit_code = run_in_namespace(|| {
        let parent_pid = unistd::getpid();

        // Create temporary zombies before calling execute
        for _ in 0..ZOMBIE_COUNT {
            #[expect(
                unsafe_code,
                reason = "Fork is required to create a zombie for testing child reaping"
            )]
            match unsafe { unistd::fork() } {
                Ok(ForkResult::Parent { child: _ }) => {}
                Ok(ForkResult::Child) => {
                    std::thread::sleep(Duration::from_millis(10));
                    std::process::exit(0);
                }
                Err(e) => return Err(format!("fork failed: {e}").into()),
            }
        }

        // NOTE: This newly created process will become zombie and the parent will not
        // reap it because the parent exits immediately while getting SIGTERM.
        #[expect(unsafe_code, reason = "Fork is required to create a child to send SIGTERM")]
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Parent { child: _ }) => {}
            Ok(ForkResult::Child) => {
                std::thread::sleep(Duration::from_millis(100));
                let _ = signal::kill(parent_pid, Signal::SIGTERM);
                std::process::exit(0);
            }
            Err(e) => return Err(format!("fork failed: {e}").into()),
        }

        ocelot_idle::execute()?;

        let zombies = find_zombie_processes()?;
        let zombie_count = zombies.len();
        if zombie_count > 1 {
            eprintln!("Zombies still present: {zombies:?}");
        }
        assert!(zombie_count <= 1, "Found {zombie_count} zombie processes");
        Ok(if zombie_count <= 1 { 0 } else { -1 })
    })?;
    assert_eq!(exit_code, 0);
    Ok(())
}
