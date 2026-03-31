use std::path::PathBuf;

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
}

/// Configuration for a single log stream (stdout or stderr).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogStreamConfig {
    pub destination: LogDestination,
    pub rotation: Option<LogRotationConfig>,
}
