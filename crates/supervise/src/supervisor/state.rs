use std::time::{Duration, Instant};

use nix::unistd::Pid;

use crate::supervisor::{Phase, RestartPolicy, dependency_registry::DependencyNotifier};

pub struct State {
    spawned: Option<Pid>,
    phase: Phase,
    ready: bool,
    restart_count: u32,
    last_exit_code: Option<i32>,
    shutdown_deadline: Option<Instant>,
    dependency_notifier: Option<DependencyNotifier>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            spawned: None,
            phase: Phase::Pending,
            ready: false,
            restart_count: 0,
            last_exit_code: None,
            shutdown_deadline: None,
            dependency_notifier: None,
        }
    }
}

impl State {
    pub const fn new(dependency_notifier: DependencyNotifier) -> Self {
        Self {
            spawned: None,
            phase: Phase::Pending,
            ready: false,
            restart_count: 0,
            last_exit_code: None,
            shutdown_deadline: None,
            dependency_notifier: Some(dependency_notifier),
        }
    }

    pub const fn set_starting(&mut self) {
        self.spawned = None;
        self.phase = Phase::Pending;
    }

    pub fn set_ready(&mut self, ready: bool) {
        self.ready = ready;
        let _ = self.dependency_notifier.as_ref().inspect(|n| n.notify_started());
    }

    pub fn set_shutting_down(&mut self, grace_period: Duration) {
        self.phase = Phase::ShuttingDown;
        self.shutdown_deadline = Some(Instant::now() + grace_period);
    }

    pub fn set_running(&mut self, spawned: Pid) {
        self.spawned = Some(spawned);
        self.phase = Phase::Running;
        self.last_exit_code = None;
        self.restart_count = 0;
        let _ = self.dependency_notifier.as_ref().inspect(|n| n.notify_started());
    }

    pub fn set_exited(&mut self, exit_code: i32) {
        self.spawned = None;
        self.last_exit_code = Some(exit_code);
        self.ready = false;
        let _ = self.dependency_notifier.as_ref().inspect(|n| n.notify_completed(exit_code));

        if exit_code == 0 {
            self.phase = Phase::Completed;
        } else {
            self.phase = Phase::CrashLoopBackOff;
        }
    }

    pub fn set_failed(&mut self, exit_code: i32) {
        self.spawned = None;
        self.last_exit_code = Some(exit_code);
        self.phase = Phase::Failed { exit_code };
        self.ready = false;
        let _ = self.dependency_notifier.as_ref().map(|n| n.notify_completed(exit_code));
    }

    pub fn notify_log_ready(&self) {
        let _ = self.dependency_notifier.as_ref().inspect(|n| n.notify_log_ready());
    }

    pub const fn process_id(&self) -> Option<Pid> { self.spawned }

    pub const fn phase(&self) -> Phase { self.phase }

    pub const fn ready(&self) -> bool { self.ready }

    pub fn shutdown_deadline_exceeded(&self) -> bool {
        self.shutdown_deadline.is_some_and(|deadline| Instant::now() >= deadline)
    }

    pub const fn clear_shutdown_deadline(&mut self) { self.shutdown_deadline = None; }

    pub const fn restart_count(&self) -> u32 { self.restart_count }

    pub const fn last_exit_code(&self) -> Option<i32> { self.last_exit_code }

    fn should_restart(&self, restart_policy: &RestartPolicy) -> bool {
        match restart_policy {
            RestartPolicy::Never => false,
            RestartPolicy::Always { .. } => true,
            RestartPolicy::OnFailure { max_retries, .. } => {
                (self.last_exit_code != Some(0)) && self.restart_count < *max_retries
            }
        }
    }

    pub fn next_interval(&mut self, restart_policy: &RestartPolicy) -> Option<Duration> {
        if self.should_restart(restart_policy) {
            self.restart_count += 1;
            self.phase = Phase::CrashLoopBackOff;
            let backoff = match restart_policy {
                RestartPolicy::Always { backoff } | RestartPolicy::OnFailure { backoff, .. } => {
                    *backoff
                }
                RestartPolicy::Never => return None,
            };

            Some(backoff)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::supervisor::{Phase, RestartPolicy, state::State};

    #[test]
    fn test_default_state() {
        let state = State::default();
        assert_eq!(state.phase(), Phase::Pending);
        assert!(!state.ready());
        assert_eq!(state.restart_count(), 0);
        assert_eq!(state.last_exit_code(), None);
        assert!(state.process_id().is_none());
    }

    #[test]
    fn test_set_starting() {
        let mut state = State::default();
        state.set_starting();
        assert_eq!(state.phase(), Phase::Pending);
        assert!(state.process_id().is_none());
    }

    #[test]
    fn test_set_ready() {
        let mut state = State::default();
        state.set_ready(true);
        assert!(state.ready());
        state.set_ready(false);
        assert!(!state.ready());
    }

    #[test]
    fn test_set_shutting_down() {
        let mut state = State::default();
        let grace_period = Duration::from_secs(30);
        state.set_shutting_down(grace_period);
        assert_eq!(state.phase(), Phase::ShuttingDown);
        assert!(!state.shutdown_deadline_exceeded());
    }

    #[test]
    fn test_clear_shutdown_deadline() {
        let mut state = State::default();
        state.set_shutting_down(Duration::from_secs(30));
        assert!(!state.shutdown_deadline_exceeded());
        state.clear_shutdown_deadline();
    }

    #[test]
    fn test_set_exited_success() {
        let mut state = State::default();
        state.set_exited(0);
        assert_eq!(state.phase(), Phase::Completed);
        assert_eq!(state.last_exit_code(), Some(0));
        assert!(!state.ready());
        assert!(state.process_id().is_none());
    }

    #[test]
    fn test_set_exited_failure() {
        let mut state = State::default();
        state.set_exited(1);
        assert_eq!(state.phase(), Phase::CrashLoopBackOff);
        assert_eq!(state.last_exit_code(), Some(1));
        assert!(!state.ready());
    }

    #[test]
    fn test_set_failed() {
        let mut state = State::default();
        state.set_failed(42);
        assert_eq!(state.phase(), Phase::Failed { exit_code: 42 });
        assert_eq!(state.last_exit_code(), Some(42));
        assert!(!state.ready());
    }

    #[test]
    fn test_phase_is_failed() {
        assert!(!Phase::Pending.is_failed());
        assert!(!Phase::Running.is_failed());
        assert!(!Phase::ShuttingDown.is_failed());
        assert!(!Phase::Completed.is_failed());
        assert!(Phase::CrashLoopBackOff.is_failed());
        assert!(Phase::Failed { exit_code: 0 }.is_failed());
        assert!(Phase::Failed { exit_code: 1 }.is_failed());
    }

    #[test]
    fn test_restart_policy_never() {
        let mut state = State::default();
        state.set_exited(1);
        let interval = state.next_interval(&RestartPolicy::Never);
        assert!(interval.is_none());
        assert_eq!(state.phase(), Phase::CrashLoopBackOff);
    }

    #[test]
    fn test_restart_policy_always() {
        let mut state = State::default();
        state.set_exited(1);
        let backoff = Duration::from_secs(5);
        let interval = state.next_interval(&RestartPolicy::Always { backoff });
        assert!(interval.is_some());
        assert_eq!(interval.unwrap(), backoff);
        assert_eq!(state.restart_count(), 1);
        assert_eq!(state.phase(), Phase::CrashLoopBackOff);
    }

    #[test]
    fn test_restart_policy_on_failure_success() {
        let mut state = State::default();
        state.set_exited(0);
        let interval = state.next_interval(&RestartPolicy::OnFailure {
            max_retries: 3,
            backoff: Duration::from_secs(1),
        });
        assert!(interval.is_none());
        assert_eq!(state.phase(), Phase::Completed);
    }

    #[test]
    fn test_restart_policy_on_failure_failure() {
        let mut state = State::default();
        state.set_exited(1);
        let interval = state.next_interval(&RestartPolicy::OnFailure {
            max_retries: 3,
            backoff: Duration::from_secs(1),
        });
        assert!(interval.is_some());
        assert_eq!(state.restart_count(), 1);
    }

    #[test]
    fn test_restart_policy_on_failure_max_retries_exceeded() {
        let mut state = State::default();
        state.set_exited(1);
        state.restart_count = 3;
        let interval = state.next_interval(&RestartPolicy::OnFailure {
            max_retries: 3,
            backoff: Duration::from_secs(1),
        });
        assert!(interval.is_none());
    }

    #[test]
    fn test_restart_policy_always_with_zero_exit() {
        let mut state = State::default();
        state.set_exited(0);
        let backoff = Duration::from_secs(5);
        let interval = state.next_interval(&RestartPolicy::Always { backoff });
        assert!(interval.is_some());
        assert_eq!(interval.unwrap(), backoff);
    }
}
