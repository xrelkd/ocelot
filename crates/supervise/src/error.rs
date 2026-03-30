use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to initialize Tokio runtime, error: {source}"))]
    InitializeTokioRuntime { source: std::io::Error },

    #[snafu(display("Failed to create signal handler, error: {source}"))]
    CreateSignalHandler { source: std::io::Error },

    #[snafu(display("Failed to spawn child process, error: {source}"))]
    SpawnChild { source: nix::Error },

    #[snafu(display("Failed to construct Pipe, error: {source}"))]
    CreatePipe { source: nix::Error },

    #[snafu(display("Failed to convert RawFd to AsyncFd, error: {source}"))]
    RegisterFd { source: std::io::Error },

    #[snafu(display("Failed to execute child process"))]
    ChildExecute,

    #[snafu(display("Failed to read from Pipe, error: {source}"))]
    ReadPipe { source: nix::Error },

    #[snafu(display("Failed to receive dependency: {source}"))]
    ReceiveDependency { source: tokio::sync::broadcast::error::RecvError },

    #[snafu(display("Failed to build splice relay, error: {source}"))]
    BuildSpliceRelay { source: crate::splice_relay::Error },
}
