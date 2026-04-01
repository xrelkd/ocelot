use std::io;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{source}"))]
    RunIdle { source: ocelot_idle::Error },

    #[snafu(display("{source}"))]
    RunEntry { source: ocelot_entry::Error },

    #[snafu(display("{source}"))]
    RunZombie { source: ocelot_zombie::Error },

    #[snafu(display("{source}"))]
    RunSupervise { source: ocelot_supervise::Error },

    #[snafu(display("{source}"))]
    RunBootstrap { source: ocelot_bootstrap::Error },

    #[snafu(display("{source}"))]
    LoadConfig { source: crate::config::Error },

    #[snafu(display("Failed to read processes, error: {source}"))]
    ReadProcesses { source: procfs::ProcError },

    #[snafu(display("Failed to write to stdout, error: {source}"))]
    WriteStdout { source: io::Error },
}

impl From<ocelot_idle::Error> for Error {
    fn from(source: ocelot_idle::Error) -> Self { Self::RunIdle { source } }
}

impl From<ocelot_entry::Error> for Error {
    fn from(source: ocelot_entry::Error) -> Self { Self::RunEntry { source } }
}

impl From<ocelot_zombie::Error> for Error {
    fn from(source: ocelot_zombie::Error) -> Self { Self::RunZombie { source } }
}

impl From<ocelot_supervise::Error> for Error {
    fn from(source: ocelot_supervise::Error) -> Self { Self::RunSupervise { source } }
}

impl From<crate::config::Error> for Error {
    fn from(source: crate::config::Error) -> Self { Self::LoadConfig { source } }
}

impl From<ocelot_bootstrap::Error> for Error {
    fn from(source: ocelot_bootstrap::Error) -> Self { Self::RunBootstrap { source } }
}
