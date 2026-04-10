//! Zombie process generator for testing process supervisors.
//!
//! This crate provides functionality to spawn child processes that exit
//! immediately, creating zombie processes. It's useful for testing how
//! process supervisors handle zombie reaping and signal handling.
//!
//! ## Overview
//!
//! The main entry point is [`execute()`], which spawns zombies at a specified
//! interval and responds to termination signals. It uses epoll for event
//! handling and signal file descriptors for asynchronous signal notification.
//!
//! ## Error Handling
//!
//! All errors are reported through the [`Error`] enum, which covers system
//! call failures, signal handling issues, and resource limitations.

mod error;

use std::time::Duration;

use nix::{
    poll::PollTimeout,
    sys::{
        epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
        signal,
        signal::{SigmaskHow, Signal},
        signalfd::{SigSet, SignalFd},
    },
    unistd,
    unistd::ForkResult,
};
use snafu::ResultExt;

pub use self::error::Error;

const SIGNAL_TOKEN: u64 = 0;

/// Executes the zombie process generator.
///
/// This function spawns child processes that exit immediately, creating zombie
/// processes at the specified interval. The parent process monitors the zombie
/// count and exits when the limit (if specified) is reached. It also responds
/// to SIGINT and SIGTERM signals for graceful shutdown.
///
/// # Arguments
///
/// * `interval` - The duration to wait between spawning zombies.
/// * `zombie_limit` - Optional maximum number of zombies to create. If `None`
///   or `Some(0)`, defaults to 5.
///
/// # Examples
///
/// ```ignore
/// # use std::time::Duration;
/// # use ocelot_zombie::{execute, Error};
/// # fn main() -> Result<(), Error> {
/// execute(Duration::from_millis(100), Some(2))?;
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// * `SetSignalMaskSnafu` - setting the signal mask fails.
/// * `CreateSignalFdSnafu` - creating the signal file descriptor fails.
/// * `CreateEpollSnafu` - creating the epoll instance fails.
/// * `AddEpollSnafu` - registering the signal fd with epoll fails.
/// * `CreateTimeoutSnafu` - converting the interval to a poll timeout fails.
/// * `WaitEpollSnafu` - waiting for epoll events fails.
/// * `ConvertU32Snafu` - converting a `u32` signal number to `i32` fails.
/// * `ParseSignalSnafu` - converting an `i32` to a `Signal` enum fails.
/// * `SpawnChildSnafu` - forking a child process fails.
pub fn execute(interval: Duration, zombie_limit: Option<u64>) -> Result<(), Error> {
    let zombie_limit = if Some(0) == zombie_limit { Some(5) } else { zombie_limit };
    let timeout = PollTimeout::try_from(interval.as_millis()).context(error::CreateTimeoutSnafu)?;

    let pid = unistd::getpid();
    tracing::info!("[Parent] PID: {pid}");

    let signal_fd = {
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGINT);
        mask.add(Signal::SIGTERM);
        signal::sigprocmask(SigmaskHow::SIG_BLOCK, Some(&mask), None)
            .context(error::SetSignalMaskSnafu)?;
        SignalFd::new(&mask).context(error::CreateSignalFdSnafu)?
    };

    let epoll = {
        let epoll = Epoll::new(EpollCreateFlags::empty()).context(error::CreateEpollSnafu)?;
        let event = EpollEvent::new(EpollFlags::EPOLLIN, SIGNAL_TOKEN);
        epoll.add(&signal_fd, event).context(error::AddEpollSnafu)?;
        epoll
    };

    let mut zombie_count = 0;
    let mut events = [EpollEvent::empty(); 1];

    'outer: loop {
        if let Some(limit) = zombie_limit
            && zombie_count >= limit
        {
            tracing::info!("[Parent] Zombie limit reached, exiting parent process {pid}");
            break;
        }

        let num_events = epoll.wait(&mut events, timeout).context(error::WaitEpollSnafu)?;
        if num_events > 0 {
            while let Ok(Some(sig_info)) = signal_fd.read_signal() {
                let sig = {
                    let signo = sig_info.ssi_signo;
                    let signal_num = i32::try_from(signo)
                        .with_context(|_| error::ConvertU32Snafu { value: signo })?;
                    Signal::try_from(signal_num)
                        .with_context(|_| error::ParseSignalSnafu { signal_num })?
                };
                match sig {
                    Signal::SIGINT | Signal::SIGTERM => {
                        tracing::info!("Shutting down, got signal: {}", sig.as_str());
                        break 'outer;
                    }
                    _ => {}
                }
            }
        } else {
            zombie_count += 1;

            #[expect(unsafe_code, reason = "Calling fork in a controlled environment.")]
            let fork_result = unsafe { unistd::fork().context(error::SpawnChildSnafu)? };

            match fork_result {
                ForkResult::Parent { child } => {
                    tracing::info!("[Parent] Spawned child PID: {child}, zombie: {zombie_count}");
                }
                ForkResult::Child => {
                    let self_pid = unistd::getpid();
                    tracing::info!("[Child {self_pid}] Exited");

                    #[expect(
                        unsafe_code,
                        reason = "Calling _exit in child process is safe after fork"
                    )]
                    unsafe {
                        nix::libc::_exit(0)
                    };
                }
            }
        }
    }

    Ok(())
}
