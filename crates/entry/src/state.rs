use std::time::Duration;

use nix::{poll::PollTimeout, unistd::Pid};
use snafu::ResultExt;

use crate::{DEFAULT_WAIT_TIMEOUT, DEFAULT_WAIT_TIMEOUT_AFTER_KILL, Error, error};

/// Tracks the state of a managed child process during supervision.
///
/// This struct maintains all state needed to track a child process lifecycle,
/// including timeout enforcement and signal handling coordination.
///
/// The state machine tracks:
/// - Whether the child has exited
/// - When signals were sent (for timeout enforcement)
/// - The final exit status
/// - Whether a kill signal has been sent
pub struct State {
    pid: Pid,
    process_exited: bool,
    status_code: i32,
    signal_time: Option<std::time::Instant>,
    timeout: Duration,
    kill_sent: bool,
}

impl State {
    /// Creates a new `State` instance for tracking a child process.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID of the child to track.
    /// * `timeout` - The duration to wait after signal before force-killing.
    ///
    /// Initially, the state marks the process as not exited with status 0.
    pub const fn new(pid: Pid, timeout: Duration) -> Self {
        Self {
            pid,
            signal_time: None,
            process_exited: false,
            status_code: 0,
            timeout,
            kill_sent: false,
        }
    }

    /// Marks the process as exited with the given status code.
    ///
    /// Once set, `is_exited()` will return `true` and `exited()` will return
    /// the stored status.
    pub const fn set_exited(&mut self, status_code: i32) {
        self.status_code = status_code;
        self.process_exited = true;
    }

    /// Marks that a kill signal has been sent to the child process.
    ///
    /// Once set, `should_force_kill()` will return `false` to prevent
    /// sending multiple kill signals.
    pub const fn set_killed(&mut self) { self.kill_sent = true; }

    /// Records the time when a signal was sent to the child.
    ///
    /// This is used to calculate timeout for graceful shutdown before
    /// force-killing. Has no effect if already set (only first signal time is
    /// tracked).
    pub fn set_signal_time(&mut self) {
        if self.signal_time.is_none() {
            self.signal_time = Some(std::time::Instant::now());
        }
    }

    /// Determines if the child should be force-killed with SIGKILL.
    ///
    /// Returns `true` if:
    /// - A signal was previously sent (`set_signal_time` was called)
    /// - The elapsed time since the signal exceeds the configured timeout
    /// - A kill has not already been sent
    ///
    /// This prevents indefinite hanging when a child ignores graceful
    /// termination.
    pub fn should_force_kill(&self) -> bool {
        if self.kill_sent {
            return false;
        }
        self.signal_time.is_some_and(|sig_time| sig_time.elapsed() >= self.timeout)
    }

    /// Checks if the process is in the process of exiting.
    ///
    /// Returns `true` if a kill signal has been sent or if a signal time
    /// has been recorded. This indicates we are in the timeout window
    /// before the child should be force-killed.
    pub const fn is_exiting(&self) -> bool { self.kill_sent || self.signal_time.is_some() }

    /// Checks if the child process has exited.
    ///
    /// Returns `true` if `set_exited` has been called, indicating we have
    /// a final exit status for the child.
    pub const fn is_exited(&self) -> bool { self.process_exited }

    /// Calculates the appropriate wait timeout for polling events.
    ///
    /// Returns a short default timeout when the child is still running
    /// normally, or a longer timeout when we're waiting for exit after
    /// sending a signal.
    ///
    /// The timeout logic:
    /// - If exiting: returns `DEFAULT_WAIT_TIMEOUT_AFTER_KILL` (200ms)
    /// - If signal time is set: returns remaining time until timeout, clamped
    ///   to `DEFAULT_WAIT_TIMEOUT`
    /// - If no signal sent: returns `DEFAULT_WAIT_TIMEOUT` (100ms)
    pub fn calculate_wait_timeout(&self) -> Duration {
        self.signal_time.map_or(DEFAULT_WAIT_TIMEOUT, |sig_time| {
            let elapsed = sig_time.elapsed();
            if elapsed >= self.timeout {
                DEFAULT_WAIT_TIMEOUT_AFTER_KILL
            } else {
                self.timeout
                    .checked_sub(elapsed)
                    .unwrap_or(DEFAULT_WAIT_TIMEOUT)
                    .min(DEFAULT_WAIT_TIMEOUT)
            }
        })
    }

    /// Calculates the epoll wait timeout, converting to `PollTimeout`.
    ///
    /// Returns `PollTimeout::NONE` (block indefinitely) if the child is still
    /// running normally. Returns a finite timeout if we're waiting for exit
    /// after sending a signal.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ConvertTimeout`] if the duration cannot be converted
    /// to `PollTimeout` (should not happen with reasonable timeout values).
    pub fn calculate_epoll_wait_timeout(&self) -> Result<PollTimeout, Error> {
        if self.is_exiting() {
            PollTimeout::try_from(self.calculate_wait_timeout()).context(error::ConvertTimeoutSnafu)
        } else {
            Ok(PollTimeout::NONE)
        }
    }

    /// Returns the process ID of the tracked child.
    pub const fn id(&self) -> Pid { self.pid }

    /// Returns the final PID and exit status code.
    ///
    /// Should only be called after `is_exited()` returns `true`. The exit
    /// status follows Unix conventions: if terminated by signal N, returns
    /// `128 + N`.
    pub const fn exited(self) -> (Pid, i32) { (self.pid, self.status_code) }
}
