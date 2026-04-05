use std::path::PathBuf;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to open console device '{}': {source}", path.display()))]
    OpenConsole { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to setup console: {source}"))]
    ConsoleSetup { source: nix::Error },

    #[snafu(display("Failed to mount {operation}: {source}"))]
    Mount { operation: String, source: nix::Error },

    #[snafu(display("Failed to switch root: {source}"))]
    SwitchRoot { source: nix::Error },

    #[snafu(display("Failed to shut down system: {source}"))]
    Shutdown { source: nix::Error },

    #[snafu(display("Failed to change working directory to '{}': {source}", path.display()))]
    FailedToChangeWorkingDirectory { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to read /proc/filesystems: {source}"))]
    ReadFilesystems { source: std::io::Error },

    #[snafu(display("Virtiofs not supported: {message}"))]
    VirtiofsNotSupported { message: &'static str },

    #[snafu(display("Failed to create directory '{}': {source}", path.display()))]
    CreateDirectory { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to create symlink '{target}' -> '{link_source}': {source}"))]
    CreateSymlink { link_source: String, target: String, source: std::io::Error },

    #[snafu(display("Failed to open kernel module '{}': {source}", path.display()))]
    OpenModule { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to decompress kernel module '{}': {source}", path.display()))]
    DecompressModule { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to create memfd for kernel module '{}': {source}", path.display()))]
    CreateMemfd { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to execute boot script: {source}"))]
    BootScript { source: ocelot_entry::Error },

    #[snafu(display("Failed to execute supervise orchestrator: {source}"))]
    ExecuteSupervise { source: ocelot_supervise::Error },

    #[snafu(display("Failed to execute interactive shell: {source}"))]
    ExecuteShell { source: ocelot_entry::Error },

    #[snafu(display("Failed to fork child process: {source}"))]
    SpawnChild { source: nix::Error },
}
