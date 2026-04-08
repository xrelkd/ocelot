use std::path::PathBuf;

use snafu::Snafu;

/// Errors that can occur during bootstrap execution.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to open console device '{}': {source}", path.display()))]
    OpenConsole { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to setup console: {source}"))]
    ConsoleSetup { source: nix::Error },

    #[snafu(display("Isolate namespace: {source}"))]
    IsolateNamespace { source: nix::Error },

    #[snafu(display("Failed to mount {} to {}: {source}", link_source.display(), target.display()))]
    Mount { link_source: PathBuf, target: PathBuf, source: nix::Error },

    #[snafu(display("Failed to unmount {}: {source}", path.display()))]
    Unmount { path: PathBuf, source: nix::Error },

    #[snafu(display("Failed to pivot root {} -> {}: {source}", old_root.display(), new_root.display()))]
    PivotRoot { new_root: PathBuf, old_root: PathBuf, source: nix::Error },

    #[snafu(display("Failed to change root directory '{}': {source}", path.display()))]
    ChangeRootDirectory { path: PathBuf, source: nix::Error },

    #[snafu(display("Failed to change directory '{}': {source}", path.display()))]
    ChangeDirectory { path: PathBuf, source: nix::Error },

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

    #[snafu(display("Failed to create symlink '{}' -> '{}': {source}", link_source.display(), target.display()))]
    CreateSymlink { link_source: PathBuf, target: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to create file '{}': {source}", path.display()))]
    CreateFile { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to set sysctl value, file '{}', value: {value}: {source}", path.display()))]
    SetSysctl { path: PathBuf, value: String, source: std::io::Error },

    #[snafu(display("Failed to open kernel module '{}': {source}", path.display()))]
    OpenModule { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to decompress kernel module '{}': {source}", path.display()))]
    DecompressModule { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to create memfd for kernel module '{}': {source}", path.display()))]
    CreateMemfd { path: PathBuf, source: std::io::Error },

    #[snafu(display("Failed to init kernel module '{}': {source}", path.display()))]
    InitializeModule { path: PathBuf, source: nix::Error },

    #[snafu(display("Failed to execute boot script: {source}"))]
    BootScript { source: ocelot_entry::Error },

    #[snafu(display("Failed to execute supervise orchestrator: {source}"))]
    ExecuteSupervise { source: ocelot_supervise::Error },

    #[snafu(display("Failed to execute interactive shell: {source}"))]
    ExecuteShell { source: ocelot_entry::Error },

    #[snafu(display("Failed to fork child process: {source}"))]
    SpawnChild { source: nix::Error },

    #[snafu(display("Hook error: {message}"))]
    Hook { message: String },
}
