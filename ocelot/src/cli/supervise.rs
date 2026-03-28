use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Subcommand;

use crate::{cli::init_tracing_subscriber, config, error::Error};

#[derive(Clone, Subcommand)]
pub enum Commands {
    #[clap(visible_alias = "r", about = "Run supervisor with configuration file")]
    Run {
        #[clap(short, long)]
        file: PathBuf,

        #[clap(long = "log-level", env = "OCELOT_LOG_LEVEL")]
        log_level: Option<tracing::Level>,
    },

    #[clap(about = "Output the configuration template in YAML format")]
    ConfigTemplate,
}

pub fn run(
    command: Option<Commands>,
    file: Option<PathBuf>,
    log_level: Option<tracing::Level>,
) -> Result<i32, Error> {
    match command {
        Some(Commands::ConfigTemplate) => {
            std::io::stdout()
                .write_all(config::SupervisorConfig::template_basic().as_slice())
                .expect("Failed to write to stdout");
            Ok(0)
        }
        Some(Commands::Run { file, log_level }) => run_supervisor(file, log_level),
        None => {
            let file = file.ok_or_else(|| Error::InvalidArgument {
                message: "missing required argument: --file <FILE>".to_owned(),
            })?;
            run_supervisor(file, log_level)
        }
    }
}

fn run_supervisor(file: impl AsRef<Path>, log_level: Option<tracing::Level>) -> Result<i32, Error> {
    let config = config::SupervisorConfig::load(file)?;
    let log_level = log_level.unwrap_or(config.log_level);
    init_tracing_subscriber(log_level);
    config.validate()?;
    let shutdown_timeout = config.shutdown_timeout_secs;
    let supervisors = config.to_supervisors();
    let config = ocelot_supervise::OrchestratorConfig {
        supervisors,
        shutdown_timeout: Duration::from_secs(shutdown_timeout),
    };
    ocelot_supervise::execute(config).map_err(Error::from)
}
