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

    #[clap(
        about = "Creates zombie processes by forking child processes that immediately exit, while \
                 the parent process sleeps. This is useful for testing how systems handle zombie \
                 processes.",
        long_about = "This command creates zombie processes by repeatedly forking child processes \
                      that immediately exit, while the parent process sleeps for a specified \
                      interval. The parent process continues to spawn new child processes until \
                      an optional limit is reached or it receives a termination signal. This is \
                      useful for testing how systems handle zombie processes and ensuring that \
                      they are properly reaped."
    )]
    Zombie {
        #[clap(
            long = "log-level",
            env = "OCELOT_LOG_LEVEL",
            default_value = "info",
            help = "Specify a log level"
        )]
        log_level: tracing::Level,

        #[arg(
            short = 'i',
            long,
            default_value = "200",
            help = "Specify a timeout in milliseconds for the zombie process to be created. The \
                    parent process will sleep for this duration after spawning each child process."
        )]
        interval_ms: u64,

        #[arg(
            short = 'c',
            long,
            help = "Specify a limit for the number of zombie processes to create. If this limit \
                    is reached, the parent process will stop spawning new child processes and \
                    exit. If not specified, it will continue to spawn zombie processes \
                    indefinitely."
        )]
        count: Option<u64>,
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
                init_tracing_subscriber(log_level);
                ocelot_idle::execute()?;
            }
            Some(Commands::Entry { log_level, commands, timeout_ms }) => {
                init_tracing_subscriber(log_level);
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
            Some(Commands::Zombie { log_level, interval_ms, count }) => {
                init_tracing_subscriber(log_level);
                let interval = Duration::from_millis(interval_ms);
                ocelot_zombie::execute(interval, count)?;
            }
            None => {
                init_tracing_subscriber(tracing::Level::INFO);
                ocelot_idle::execute()?;
            }
        }
        Ok(0)
    }
}

fn init_tracing_subscriber(log_level: tracing::Level) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.as_str())),
        )
        .init();
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
