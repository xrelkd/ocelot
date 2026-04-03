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
    pub log_stdout: LogStreamConfig,
    pub log_stderr: LogStreamConfig,
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
            log_stdout: LogStreamConfig { destination: LogDestination::Inherit, rotation: None },
            log_stderr: LogStreamConfig { destination: LogDestination::Inherit, rotation: None },
        }
    }
}

impl Config {
    #[must_use]
    pub fn name(&self) -> &str { &self.name }

    #[must_use]
    pub fn command(&self) -> Command {
        let mut cmd =
            Command::new(&self.program).args(&self.arguments).envs(&self.environment_variables);
        if let Some(dir) = &self.working_directory {
            cmd = cmd.current_dir(dir);
        }
        if matches!(self.log_stdout.destination, LogDestination::Null) {
            cmd = cmd.discard_stdout(true);
        }
        if matches!(self.log_stderr.destination, LogDestination::Null) {
            cmd = cmd.discard_stderr(true);
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

/// Compression algorithm for rotated log files.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum LogCompression {
    #[default]
    None,
    Gzip,
    Lz4,
}

/// Destination for log output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LogDestination {
    Null,
    Inherit,
    File { path: PathBuf },
}

/// Rotation configuration for log files.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogRotationConfig {
    pub max_size_bytes: Option<u64>,
    pub rotation_interval_secs: Option<u64>,
    pub max_files: Option<u32>,
    pub max_age_days: Option<u32>,
    pub mode: Option<u32>,
    pub compression: LogCompression,
}

/// Configuration for a single log stream (stdout or stderr).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogStreamConfig {
    pub destination: LogDestination,
    pub rotation: Option<LogRotationConfig>,
}
