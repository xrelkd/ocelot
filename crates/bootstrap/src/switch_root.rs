use std::ffi::CString;

use nix::{libc, unistd};

use crate::{ShellConfig, mount};

/// Performs `switch_root`: move mounts, chroot, and hand off to supervise.
///
/// After this call the process is running in the new root filesystem
/// and the supervise orchestrator takes over.
pub fn switch_root(
    orchestrator_config: ocelot_supervise::OrchestratorConfig,
) -> Result<(), nix::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot")?;
    unistd::chroot(".")?;
    unistd::chdir("/")?;

    let _ = ocelot_supervise::execute(orchestrator_config).map_err(|_| nix::Error::EIO)?;
    Ok(())
}

/// Performs `switch_root` and spawns an interactive shell.
///
/// After this call the process is running in the new root filesystem
/// and a shell is spawned with the console as controlling terminal.
///
/// This function never returns on success — the shell is exec'd.
pub fn switch_root_shell(console: &str, shell_config: &ShellConfig) -> Result<(), nix::Error> {
    mount::mount_move_special()?;

    unistd::chdir("/newroot")?;
    unistd::chroot(".")?;
    unistd::chdir("/")?;

    // Open the console device
    let console_path =
        if console.starts_with('/') { console.to_string() } else { format!("/dev/{console}") };

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&console_path)
        .map_err(|_| nix::Error::ENOENT)?;

    // Create a new session
    let _ = unistd::setsid()?;

    // Dup console to stdin, stdout, stderr
    #[expect(unsafe_code, reason = "dup2_raw is safe with valid file descriptor")]
    unsafe {
        let _stdin = unistd::dup2_raw(&file, libc::STDIN_FILENO)?;
        let _stdout = unistd::dup2_raw(&file, libc::STDOUT_FILENO)?;
        let _stderr = unistd::dup2_raw(&file, libc::STDERR_FILENO)?;
    }

    // Set controlling terminal via TIOCSCTTY
    #[expect(unsafe_code, reason = "ioctl TIOCSCTTY is safe after setsid and dup2")]
    unsafe {
        let _ = libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY, 0);
    }

    // Exec into the shell
    let program_c = CString::new(shell_config.program.as_str()).map_err(|_| nix::Error::EINVAL)?;
    let mut argv: Vec<CString> = vec![program_c.clone()];
    for arg in &shell_config.args {
        argv.push(CString::new(arg.as_str()).map_err(|_| nix::Error::EINVAL)?);
    }
    let argv_refs: Vec<&CString> = argv.iter().collect();

    unistd::execv(&program_c, &argv_refs).map_err(|_| nix::Error::EIO)?;

    // execv only returns on error
    Ok(())
}
