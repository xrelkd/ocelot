use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{source}"))]
    RunIdle { source: ocelot_idle::Error },

    #[snafu(display("{source}"))]
    RunEntry { source: ocelot_entry::Error },
}

impl From<ocelot_idle::Error> for Error {
    fn from(source: ocelot_idle::Error) -> Self { Self::RunIdle { source } }
}

impl From<ocelot_entry::Error> for Error {
    fn from(source: ocelot_entry::Error) -> Self { Self::RunEntry { source } }
}
