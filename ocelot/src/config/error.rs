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

#[derive(Debug, Snafu)]
pub enum ValidationError {
    #[snafu(display("Cyclic dependency detected involving process '{process}'"))]
    CyclicDependency { process: String },

    #[snafu(display("Process '{process}' depends on non-existent process '{depends_on}'"))]
    MissingDependency { process: String, depends_on: String },

    #[snafu(display("Unsupported config version '{version}'"))]
    InvalidVersion { version: String },
}
