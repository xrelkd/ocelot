use std::io::Write;

use clap::{CommandFactory, Parser, Subcommand};

use crate::{error::Error, shadow};

#[derive(Parser)]
#[command(
    name = "ocelot",
    author,
    version,
    long_version = shadow::CLAP_LONG_VERSION,
    about,
    long_about = None
)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Option<Commands>,
}

#[allow(variant_size_differences)]
#[derive(Clone, Subcommand)]
pub enum Commands {
    #[clap(about = "Print the version information")]
    Version,

    #[clap(about = "Output shell completion code for the specified shell (bash, zsh, fish)")]
    Completions { shell: clap_complete::Shell },

    #[clap(
        about = "Run as a minimalist PID 1 to reap zombies and hold namespaces",
        long_about = "Acts as a 'pause' container equivalent. It enters an infinite loop waiting \
                      for signals. When SIGCHLD is received, it reaps exited child processes to \
                      prevent zombies. This is essential when running in environments where this \
                      process is the sub-grid anchor (PID 1)."
    )]
    Noop {
        #[clap(
            long = "log-level",
            default_value = "info",
            env = "OCELOT_LOG_LEVEL",
            help = "Specify a log level"
        )]
        log_level: tracing::Level,
    },
}

impl Default for Cli {
    fn default() -> Self { Self::parse() }
}

impl Cli {
    pub fn run(self) -> Result<i32, Error> {
        match self.commands {
            Some(Commands::Version) => {
                std::io::stdout()
                    .write_all(Self::command().render_long_version().as_bytes())
                    .expect("Failed to write to stdout");
            }
            Some(Commands::Completions { shell }) => {
                let mut app = Self::command();
                let bin_name = app.get_name().to_string();
                clap_complete::generate(shell, &mut app, bin_name, &mut std::io::stdout());
            }
            Some(Commands::Noop { log_level }) => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| tracing_subscriber::EnvFilter::new(log_level.as_str()),
                        ),
                    )
                    .init();
                ocelot_noop::execute()?;
            }
            None => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| tracing_subscriber::EnvFilter::new(tracing::Level::INFO.as_str()),
                        ),
                    )
                    .init();
                ocelot_noop::execute()?;
            }
        }
        Ok(0)
    }
}
