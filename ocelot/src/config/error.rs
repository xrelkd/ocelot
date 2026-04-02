use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to resolve config file path: {}", file_path.display()))]
    ResolveFilePath { file_path: std::path::PathBuf },

    #[snafu(display("Failed to open config file: {}", filename.display()))]
    OpenConfig { filename: std::path::PathBuf, source: std::io::Error },

    #[snafu(display("Failed to parse config file: {}, error: {source}", filename.display()))]
    ParseConfig { filename: std::path::PathBuf, source: serde_yaml::Error },

    #[snafu(display("Configuration validation failed: {source}"))]
    Validate { source: ValidationError },
}

impl From<ValidationError> for Error {
    fn from(source: ValidationError) -> Self { Self::Validate { source } }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum ValidationError {
    #[snafu(display("Cyclic dependency detected: {}",
        cycle.join(" → ")))]
    CyclicDependency { cycle: Vec<String> },

    #[snafu(display("Process '{process}' depends on non-existent process '{depends_on}'"))]
    MissingDependency { process: String, depends_on: String },

    #[snafu(display("Unsupported config version '{version}'"))]
    InvalidVersion { version: String },

    #[snafu(display("Log rotation field '{field}' must be positive, got {value}"))]
    InvalidLogRotation { field: String, value: i64 },

    #[snafu(display("Probe timeout ({timeout}s) must not exceed period ({period}s)"))]
    InvalidProbeTimeout { timeout: u64, period: u64 },

    #[snafu(display("Probe port {port} is out of valid range (1-65535)"))]
    InvalidProbePort { port: u16 },

    #[snafu(display("Restart policy backoff must be positive, got {backoff}s"))]
    InvalidRestartBackoff { backoff: u64 },

    #[snafu(display("Termination grace period must be positive, got {value}s"))]
    InvalidTerminationGracePeriod { value: u64 },

    #[snafu(display("Process '{process}' is missing required 'program' field"))]
    MissingProcessProgram { process: String },

    #[snafu(display("Process '{process}' has duplicate environment variables: {}",
        variables.iter().map(String::as_str).collect::<Vec<_>>().join(", ")))]
    DuplicateEnvironmentVariables { process: String, variables: Vec<String> },

    #[snafu(display("Invalid rotation configuration: {reason}"))]
    InvalidRotationConfiguration { reason: String },
}
