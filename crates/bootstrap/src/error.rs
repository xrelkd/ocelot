use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to open console device '{path}': {source}"))]
    OpenConsole { path: String, source: std::io::Error },

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

    #[snafu(display("Failed to read /proc/filesystems: {source}"))]
    ReadFilesystems { source: std::io::Error },

    #[snafu(display("Virtiofs not supported: {message}"))]
    VirtiofsNotSupported { message: &'static str },

    #[snafu(display("Failed to create directory '{path}': {source}"))]
    CreateDirectory { path: String, source: std::io::Error },

    #[snafu(display("Failed to create symlink '{target}' -> '{link_source}': {source}"))]
    CreateSymlink { link_source: String, target: String, source: std::io::Error },

    #[snafu(display("Failed to open kernel module '{path}': {source}"))]
    OpenModule { path: String, source: std::io::Error },

    #[snafu(display("Failed to execute boot script: {source}"))]
    BootScript { source: ocelot_entry::Error },

    #[snafu(display("Failed to execute supervise orchestrator: {source}"))]
    ExecuteSupervise { source: ocelot_supervise::Error },

    #[snafu(display("Failed to execute interactive shell: {source}"))]
    ExecuteShell { source: ocelot_entry::Error },
}
