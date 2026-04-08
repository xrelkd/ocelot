use snafu::ResultExt;

use crate::{error, error::Error};

/// Scans the system for zombie processes and reports findings.
///
/// This function iterates through all processes using procfs, identifies
/// zombie processes (those with state 'Z'), prints their PID and command
/// name, and returns the count of zombies found.
///
/// # Errors
/// * `ReadProcesses` - if procfs process enumeration fails
///
/// # Examples
/// ```
/// use ocelot::cli::zombie_finder::run;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let result = run()?;
/// assert_eq!(result, 0);
/// # Ok(())
/// # }
/// ```
pub fn run() -> Result<i32, Error> {
    let count = procfs::process::all_processes()
        .map(|procs| {
            procs
                .flatten()
                .filter_map(|p| p.stat().ok())
                .filter_map(|stat| {
                    (stat.state == 'Z').then_some(ZombieProcess { pid: stat.pid, comm: stat.comm })
                })
                .inspect(|z| println!("[ZOMBIE FOUND] PID: {:<6} Command: {}", z.pid, z.comm))
        })
        .context(error::ReadProcessesSnafu)?
        .count();
    let _ = (count == 0).then(|| eprintln!("System is clean, no zombie processes found."));
    Ok(0)
}

/// Represents a zombie process found during scanning.
///
/// Contains the process ID and command name of a zombie process.
struct ZombieProcess {
    /// The process ID of the zombie process.
    pub pid: i32,
    /// The command name of the zombie process.
    pub comm: String,
}
