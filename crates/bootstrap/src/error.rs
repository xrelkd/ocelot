use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to setup console: {source}"))]
    ConsoleSetup { source: nix::Error },

    #[snafu(display("Failed to mount {operation}: {source}"))]
    Mount { operation: String, source: nix::Error },

    #[snafu(display("Failed to switch root: {source}"))]
    SwitchRoot { source: nix::Error },

    #[snafu(display("Failed to shut down system: {source}"))]
    Shutdown { source: nix::Error },
}
