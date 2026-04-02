use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use clap::Subcommand;
use snafu::ResultExt;

use crate::{cli::init_tracing_subscriber, config, error, error::Error};

#[derive(Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

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

    #[clap(about = "Validate the configuration file")]
    Validate {
        file: PathBuf,
        #[clap(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub fn run(
    command: Option<Commands>,
    file: PathBuf,
    log_level: Option<tracing::Level>,
) -> Result<i32, Error> {
    match command {
        Some(Commands::ConfigTemplate) => {
            std::io::stdout()
                .write_all(config::SuperviseConfig::template_basic().as_slice())
                .context(error::WriteStdoutSnafu)?;
            Ok(0)
        }
        Some(Commands::Validate { file, output }) => Ok(validate_config(&file, output)),
        Some(Commands::Run { file, log_level }) => run_supervisor(file, log_level),
        None => run_supervisor(file, log_level),
    }
}

fn run_supervisor(file: impl AsRef<Path>, log_level: Option<tracing::Level>) -> Result<i32, Error> {
    let config = config::SuperviseConfig::load(file)?;
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

fn validate_config(file: &PathBuf, output: OutputFormat) -> i32 {
    // Load config
    let cfg = match config::SuperviseConfig::load(file) {
        Ok(cfg) => cfg,
        Err(e) => {
            print_error(&e, output);
            return 1;
        }
    };

    // Validate
    if let Err(e) = cfg.validate() {
        print_error(&e, output);
        return 1;
    }

    // Success
    if output == OutputFormat::Human {
        println!("Configuration is valid");
    } else {
        println!("{{\"valid\":true}}");
    }
    0
}

fn print_error(e: &dyn std::fmt::Display, output: OutputFormat) {
    let message = e.to_string();
    if output == OutputFormat::Human {
        eprintln!("{message}");
    } else {
        match serde_json::to_string_pretty(&serde_json::json!({
            "valid": false,
            "errors": [{"message": message}]
        })) {
            Ok(json) => eprintln!("{json}"),
            Err(_) => eprintln!("{{\"valid\":false,\"errors\":[{{\"message\":\"{message}\"}}]}}"),
        }
    }
}
