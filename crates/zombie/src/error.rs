use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create signal handler, error: {source}"))]
    CreateSignalHandler { source: std::io::Error },

    #[snafu(display("Failed to spawn child process, error: {source}"))]
    SpawnChild { source: nix::Error },
}
