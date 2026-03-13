use std::{io::Write, time::Duration};

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
        visible_aliases = ["noop", "pause"],
        about = "Run as a minimalist PID 1 to reap zombies and hold namespaces",
        long_about = "Acts as a 'pause' container equivalent. It enters an infinite loop waiting \
                      for signals. When SIGCHLD is received, it reaps exited child processes to \
                      prevent zombies. This is essential when running in environments where this \
                      process is the sub-grid anchor (PID 1)."
    )]
    Idle {
        #[clap(
            long = "log-level",
            env = "OCELOT_LOG_LEVEL",
            default_value = "info",
            help = "Specify a log level"
        )]
        log_level: tracing::Level,
    },

    #[clap(
        visible_aliases = ["wrap"],
        about = "Spawns and supervises a child process as a minimalist PID 1 with signal forwarding and zombie reaping",
        long_about = "Acts as a process supervisor and init system for containerized workloads. It forks and executes a child process, then assumes responsibility for the PID 1 lifecycle. It ensures system stability by proactively reaping zombie processes via SIGCHLD and proxies termination signals (SIGINT/SIGTERM) to the child. If the child fails to exit within a grace period, it enforces a SIGKILL to ensure the container terminates. This is essential for preventing process leaks and ensuring clean shutdowns in orchestrated environments."
    )]
    Entry {
        #[clap(
            long = "log-level",
            env = "OCELOT_LOG_LEVEL",
            default_value = "info",
            help = "Specify a log level"
        )]
        log_level: tracing::Level,

        #[arg(
            long,
            help = "Specify a timeout in milliseconds for the command to execute. If the command \
                    does not finish within the specified timeout, it will be forcefully killed."
        )]
        timeout_ms: Option<u64>,

        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        commands: Vec<String>,
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
            Some(Commands::Idle { log_level }) => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| tracing_subscriber::EnvFilter::new(log_level.as_str()),
                        ),
                    )
                    .init();
                ocelot_idle::execute()?;
            }
            Some(Commands::Entry { log_level, commands, timeout_ms }) => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| tracing_subscriber::EnvFilter::new(log_level.as_str()),
                        ),
                    )
                    .init();
                let (command, args) = if commands.is_empty() {
                    tracing::warn!("No command provided, nothing to execute.");
                    return Ok(0);
                } else {
                    let (command, args) =
                        commands.split_first().expect("Just checked it's not empty");
                    let command = command.clone();
                    let args = args.to_vec();
                    (command, args)
                };
                let timeout = timeout_ms.map(Duration::from_millis);
                return ocelot_entry::execute(command, args, timeout).map_err(Error::from);
            }
            None => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
                            |_| tracing_subscriber::EnvFilter::new(tracing::Level::INFO.as_str()),
                        ),
                    )
                    .init();
                ocelot_idle::execute()?;
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::{Cli, Commands};

    #[test]
    fn test_command_simple() {
        if matches!(Cli::parse_from(["program_name", "version"]).commands, Some(Commands::Version))
        {
            // everything is good.
        } else {
            panic!();
        }
    }
}
