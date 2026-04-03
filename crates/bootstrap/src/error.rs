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

    #[snafu(display("Failed to change working directory to '{path}': {source}"))]
    FailedToChangeWorkingDirectory { path: String, source: std::io::Error },

    #[snafu(display("Virtiofs not supported: {message}"))]
    VirtiofsNotSupported { message: String },

    #[snafu(display("Failed to execute boot script: {source}"))]
    BootScript { source: ocelot_entry::Error },

    #[snafu(display("Failed to execute supervise orchestrator: {source}"))]
    ExecuteSupervise { source: ocelot_supervise::Error },

    #[snafu(display("Failed to execute interactive shell: {source}"))]
    ExecuteShell { source: ocelot_entry::Error },
}
