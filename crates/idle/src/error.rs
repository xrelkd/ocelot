use snafu::Snafu;

/// Errors that can occur when setting up the idle process supervisor.
///
/// The idle process handles signals and reaps zombies. This error type
/// represents failures during initialization, primarily signal handler setup.
///
/// # Variants
///
/// * `CreateSignalHandler` - Failed to create signal handler due to I/O error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create signal handler, error: {source}"))]
    CreateSignalHandler { source: std::io::Error },
}
