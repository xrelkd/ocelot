use std::time::Duration;

use crate::supervisor::SupervisorConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub supervisors: Vec<SupervisorConfig>,
    pub shutdown_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self { supervisors: Vec::new(), shutdown_timeout: Duration::from_secs(30) }
    }
}
