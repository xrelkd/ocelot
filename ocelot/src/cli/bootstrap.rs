//! Bootstrap subcommand handler.
//!
//! This module handles the `ocelot bootstrap` (alias: `boot`) subcommand,
//! which acts as an initramfs init system for QEMU VMs.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use clap::Subcommand;
use snafu::ResultExt;

use crate::{
    config::{BootstrapConfig, HandoffMode},
    error,
    error::Error,
};

#[derive(Clone, Subcommand)]
pub enum Commands {
    #[clap(visible_alias = "r", about = "Run bootstrap with configuration file")]
    Run {
        #[clap(short, long)]
        file: PathBuf,
    },

    #[clap(about = "Output the configuration template in YAML format")]
    ConfigTemplate {
        #[clap(long, default_value = "shell")]
        mode: TemplateMode,
    },

    #[clap(about = "Validate the configuration file")]
    Validate {
        file: PathBuf,

        #[clap(long, default_value = "human")]
        output: OutputFormat,
    },
}

#[derive(Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum TemplateMode {
    Shell,
    Supervise,
}

#[derive(Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

pub fn run(command: Option<Commands>, file: Option<PathBuf>) -> Result<i32, Error> {
    match command {
        Some(Commands::ConfigTemplate { mode }) => {
            let template_bytes = match mode {
                TemplateMode::Shell => BootstrapConfig::template_shell(),
                TemplateMode::Supervise => BootstrapConfig::template_supervise(),
            };
            std::io::stdout()
                .write_all(template_bytes.as_slice())
                .context(error::WriteStdoutSnafu)?;
            Ok(0)
        }
        Some(Commands::Validate { file, output }) => Ok(validate_config(&file, output)),
        Some(Commands::Run { file }) => run_bootstrap(file),
        None => {
            let path = file.unwrap_or_else(|| {
                ocelot_bootstrap::get_config_path()
                    .map_or_else(|| PathBuf::from("/etc/ocelot/bootstrap.yaml"), PathBuf::from)
            });
            run_bootstrap(path)
        }
    }
}

fn run_bootstrap(path: impl AsRef<Path>) -> Result<i32, Error> {
    let mut config = BootstrapConfig::load(path)?;
    // Validate and reorder module load order based on dependencies
    config.validate()?;
    let handoff_mode = config.handoff_mode();
    let log_level = config.log_level;
    let bootstrap_config = ocelot_bootstrap::Config::from(config);
    match handoff_mode {
        HandoffMode::Shell => {
            init_tracing_subscriber(tracing::Level::INFO);
            ocelot_bootstrap::execute_shell(&bootstrap_config)?;
            ocelot_bootstrap::shutdown()?;
        }
        HandoffMode::Supervise => {
            init_tracing_subscriber(log_level);
            ocelot_bootstrap::execute_supervise(&bootstrap_config)?;
            ocelot_bootstrap::shutdown()?;
        }
    }
    Ok(0)
}

fn validate_config(file: &PathBuf, output: OutputFormat) -> i32 {
    let mut cfg = match BootstrapConfig::load(file) {
        Ok(cfg) => cfg,
        Err(e) => {
            print_error(&e, output);
            return 1;
        }
    };

    if let Err(e) = cfg.validate() {
        print_error(&e, output);
        return 1;
    }

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

fn init_tracing_subscriber(log_level: tracing::Level) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.as_str())),
        )
        .init();
}
