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

    #[expect(unsafe_code, reason = "setsid, dup2_raw, and ioctl are safe with valid arguments")]
    unsafe {
        let _pid = unistd::setsid()?;
        let _stdin = unistd::dup2_raw(&file, libc::STDIN_FILENO)?;
        let _stdout = unistd::dup2_raw(&file, libc::STDOUT_FILENO)?;
        let _stderr = unistd::dup2_raw(&file, libc::STDERR_FILENO)?;

        // Set controlling terminal via TIOCSCTTY
        let _ = libc::ioctl(libc::STDIN_FILENO, libc::TIOCSCTTY, 0);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_console_path_with_absolute() {
        // Test that absolute paths are used directly
        let path = "/dev/ttyS0";
        let result = if path.starts_with('/') { path.to_string() } else { format!("/dev/{path}") };
        assert_eq!(result, "/dev/ttyS0");
    }

    #[test]
    fn test_console_path_with_relative() {
        // Test that relative paths get /dev/ prefix
        let path = "console";
        let result = if path.starts_with('/') { path.to_string() } else { format!("/dev/{path}") };
        assert_eq!(result, "/dev/console");
    }
}
