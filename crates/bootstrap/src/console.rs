use std::fs::OpenOptions;

use nix::{libc, unistd};

/// Sets up the console device for standard I/O.
///
/// Opens the specified console device and duplicates it to stdin, stdout, and
/// stderr. Also creates a new session with `setsid()`.
pub fn setup(console_device: &str) -> Result<(), nix::Error> {
    let path = if console_device.starts_with('/') {
        console_device.to_string()
    } else {
        format!("/dev/{console_device}")
    };

    let file =
        OpenOptions::new().read(true).write(true).open(&path).map_err(|_| nix::Error::ENOENT)?;

    #[expect(unsafe_code, reason = "setsid and dup2_raw are safe with valid arguments")]
    unsafe {
        let _pid = unistd::setsid()?;
        let _stdin = unistd::dup2_raw(&file, libc::STDIN_FILENO)?;
        let _stdout = unistd::dup2_raw(&file, libc::STDOUT_FILENO)?;
        let _stderr = unistd::dup2_raw(&file, libc::STDERR_FILENO)?;
    }

    Ok(())
}
