use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    InvalidInput {
        input: String,
        source: std::ffi::NulError,
    },

    #[snafu(display("Failed to create signal handler, error: {source}"))]
    CreateSignalHandler {
        source: std::io::Error,
    },

    #[snafu(display("Failed to spawn child process, error: {source}"))]
    SpawnChild {
        source: nix::Error,
    },

    #[snafu(display("Failed to kill child process, error: {source}"))]
    KillChild {
        source: std::io::Error,
    },

    #[snafu(display("Failed to wait for child process, error: {source}"))]
    WaitChild {
        source: std::io::Error,
    },

    #[snafu(display("Failed to wait for child process (nix), error: {source}"))]
    WaitPid {
        source: nix::Error,
    },
}
