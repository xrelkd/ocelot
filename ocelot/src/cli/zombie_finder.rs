use snafu::ResultExt;

use crate::{error, error::Error};

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

struct ZombieProcess {
    pub pid: i32,
    pub comm: String,
}
