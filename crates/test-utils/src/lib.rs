use std::path::Path;

use nix::{mount, sched, unistd};

/// Test result type used throughout ocelot tests.
///
/// A specialized `Result` type that uses `Box<dyn std::error::Error>` as the
/// error type, providing flexibility for any error that implements the `Error`
/// trait.
pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Scan all processes and return those in zombie state (state='Z').
///
/// # Errors
///
/// Returns `Err` if:
/// - Reading the process list from `/proc` fails (e.g., permission denied)
/// - Any individual process cannot be read or parsed (such errors are silently
///   ignored via `ok()` and that process is skipped)
///
/// # Examples
///
/// ```ignore
/// use ocelot_test_utils::find_zombie_processes;
///
/// let zombies = find_zombie_processes().unwrap();
/// println!("Found {} zombie processes", zombies.len());
/// ```
pub fn find_zombie_processes() -> TestResult<Vec<i32>> {
    Ok(procfs::process::all_processes()?
        .filter_map(|proc_entry| {
            proc_entry
                .ok()
                .and_then(|proc| proc.stat().ok())
                .filter(|stat| stat.state == 'Z')
                .map(|stat| stat.pid)
        })
        .collect())
}

/// Check if user namespaces are supported by forking a child and trying
/// unshare in that child. Returns true if supported, false otherwise.
///
/// This function performs a fork and attempts to create a new user namespace
/// in the child process. If the operation succeeds, the parent waits for the
/// child to exit and returns `true`. If any step fails (fork error, permission
/// denied, etc.), it returns `false`.
///
/// # Examples
/// ```ignore
/// use ocelot_test_utils::supports_user_namespace;
///
/// if !supports_user_namespace() {
///     eprintln!("User namespaces not supported, skipping test");
///     return;
/// }
/// ```
#[must_use]
pub fn supports_user_namespace() -> bool {
    #[expect(unsafe_code, reason = "Fork is safe in single-threaded context")]
    match unsafe { unistd::fork() } {
        Ok(unistd::ForkResult::Parent { child }) => {
            matches!(
                nix::sys::wait::waitpid(child, None),
                Ok(nix::sys::wait::WaitStatus::Exited(_, 0))
            )
        }
        Ok(unistd::ForkResult::Child) => {
            let result = sched::unshare(sched::CloneFlags::CLONE_NEWUSER);
            std::process::exit(match result {
                Ok(()) => 0,
                Err(_) => 1,
            });
        }
        Err(_) => false,
    }
}

/// Run a test function inside a user+mount+PID namespace.
///
/// Returns the exit code of the grandchild process that runs the test.
/// This uses a double-fork to ensure the test runs as PID 1 (init) in the new
/// namespace.
///
/// # Errors
///
/// Returns `Err` if:
/// - User namespaces are not supported on this system
/// - The first fork fails to create child A
/// - Child A's `unshare` call fails (insufficient permissions or kernel
///   support)
/// - The second fork fails to create child B
/// - UID/GID mapping fails (writing to `/proc/self/uid_map` or
///   `/proc/self/gid_map`)
/// - Proc mount fails
/// - Child A encounters an unexpected wait status
/// # Examples
///
/// ```ignore
/// use ocelot_test_utils::{run_in_namespace, supports_user_namespace};
///
/// if !supports_user_namespace() {
///     return;
/// }
///
/// let _exit_code = run_in_namespace(|| {
///     // Code that needs to run as PID 1 in a new namespace
///     println!("Running as PID {}", nix::unistd::getpid());
///     Ok(0)
/// }).unwrap();
/// ```
#[expect(
    unsafe_code,
    reason = "Namespace isolation testing requires forking, which uses unsafe blocks"
)]
pub fn run_in_namespace<F>(test_fn: F) -> TestResult<i32>
where
    F: FnOnce() -> TestResult<i32> + Send + 'static,
{
    if !supports_user_namespace() {
        return Err("user namespaces not supported".into());
    }

    let uid = unistd::getuid();
    let gid = unistd::getgid();

    // First fork: create child A
    let fork1 = unsafe { unistd::fork() };
    match fork1 {
        Ok(unistd::ForkResult::Parent { child, .. }) => {
            // Parent: wait for child A to exit and return its status
            let status = nix::sys::wait::waitpid(child, None)?;
            match status {
                nix::sys::wait::WaitStatus::Exited(_, code) => Ok(code),
                nix::sys::wait::WaitStatus::Signaled(_, sig, _) => Ok(128 + sig as i32),
                _ => Err(format!("Unexpected wait status: {status:?}").into()),
            }
        }
        Ok(unistd::ForkResult::Child) => {
            // Child A: create new namespaces
            sched::unshare(
                sched::CloneFlags::CLONE_NEWUSER
                    | sched::CloneFlags::CLONE_NEWNS
                    | sched::CloneFlags::CLONE_NEWPID,
            )?;

            // Second fork: child B becomes PID 1 in the new namespace
            let fork2 = unsafe { unistd::fork() };
            match fork2 {
                Ok(unistd::ForkResult::Parent { child, .. }) => {
                    // Child A waits for child B and exits with its status
                    let status = nix::sys::wait::waitpid(child, None)?;
                    let exit_code = match status {
                        nix::sys::wait::WaitStatus::Exited(_, code) => code,
                        nix::sys::wait::WaitStatus::Signaled(_, sig, _) => 128 + sig as i32,
                        _ => 1,
                    };
                    std::process::exit(exit_code);
                }
                Ok(unistd::ForkResult::Child) => {
                    // Child B: this will be PID 1
                    setup_uid_gid_map(uid, gid)?;
                    setup_proc_mount()?;
                    let exit_code = match test_fn() {
                        Ok(code) => code,
                        Err(e) => {
                            eprintln!("Test error: {e}");
                            1
                        }
                    };
                    std::process::exit(exit_code);
                }
                Err(e) => {
                    eprintln!("Second fork failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        Err(e) => Err(format!("first fork failed: {e}").into()),
    }
}

// Write UID/GID maps to map current user to root (0) inside the user
// namespace.
fn setup_uid_gid_map(uid: unistd::Uid, gid: unistd::Gid) -> TestResult<()> {
    std::fs::write("/proc/self/uid_map", format!("0 {uid} 1"))?;
    drop(std::fs::write("/proc/self/setgroups", "deny"));
    std::fs::write("/proc/self/gid_map", format!("0 {gid} 1"))?;
    Ok(())
}

// Re-mount /proc inside the new mount namespace.
fn setup_proc_mount() -> TestResult<()> {
    mount::mount(
        Some("proc"),
        Path::new("/proc"),
        Some("proc"),
        mount::MsFlags::MS_NOSUID | mount::MsFlags::MS_NODEV | mount::MsFlags::MS_NOEXEC,
        None::<&str>,
    )?;
    Ok(())
}
