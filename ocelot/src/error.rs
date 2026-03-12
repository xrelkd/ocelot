use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("{source}"))]
    RunNoop { source: ocelot_noop::Error },
}

impl From<ocelot_noop::Error> for Error {
    fn from(source: ocelot_noop::Error) -> Self { Self::RunNoop { source } }
}
