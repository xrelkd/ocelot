use std::{collections::HashMap, path::PathBuf, time::Duration};

use nix::sys::signal::Signal;

use crate::{Command, supervisor::probe::Probe};

#[derive(Clone, Debug)]
pub struct Config {
    pub name: String,
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub working_directory: Option<PathBuf>,
    pub depends_on: HashMap<String, ProcessDependency>,

    pub readiness_probe: Option<Probe>,
    pub liveness_probe: Option<Probe>,
    pub restart_policy: RestartPolicy,
    pub shutdown_signal: Option<Signal>,
    pub termination_grace_period: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: String::new(),
            program: PathBuf::new(),
            arguments: Vec::new(),
            environment_variables: HashMap::new(),
            working_directory: None,
            depends_on: HashMap::new(),
            readiness_probe: None,
            liveness_probe: None,
            restart_policy: RestartPolicy::default(),
            shutdown_signal: None,
            termination_grace_period: Duration::from_secs(30),
        }
    }
}

impl Config {
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    #[must_use]
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(self.program.clone())
            .args(self.arguments.clone())
            .envs(self.environment_variables.clone());
        if let Some(dir) = &self.working_directory {
            cmd = cmd.current_dir(dir.clone());
        }
        cmd
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum RestartPolicy {
    #[default]
    Never,
    Always {
        backoff: Duration,
    },
    OnFailure {
        max_retries: u32,
        backoff: Duration,
    },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessDependency {
    pub condition: Option<DependencyCondition>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DependencyCondition {
    #[default]
    Started,
    Healthy,
    Completed,
    CompletedSuccessfully,
    LogReady,
}
