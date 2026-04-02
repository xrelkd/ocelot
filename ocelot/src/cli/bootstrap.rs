//! Bootstrap subcommand handler.
//!
//! This module handles the `ocelot bootstrap` (alias: `boot`) subcommand,
//! which acts as an initramfs init system for QEMU VMs.

use std::path::Path;

use crate::{config::BootstrapConfig, error::Error};

/// Runs the bootstrap subcommand.
///
/// Loads the bootstrap configuration file, initializes logging based on
/// the config, and executes the bootstrap flow in either shell or supervise
/// mode.
///
/// For shell mode, logging is always initialized at `info` level to show
/// important information. For supervise mode, the log level from the
/// config file is used.
///
/// # Arguments
/// * `path` - Path to the bootstrap YAML configuration file
///
/// # Returns
/// * `Ok(0)` on success (never returns on successful bootstrap)
/// * `Err(Error)` if configuration loading or execution fails
pub fn run(path: impl AsRef<Path>) -> Result<i32, Error> {
    let config = BootstrapConfig::load(path)?;
    let bootstrap_config = config.to_bootstrap_config();

    // Initialize logging after parsing config
    if let Some(shell_config) = config.to_shell_config() {
        // Shell mode for debugging
        init_tracing_subscriber(tracing::Level::INFO);
        ocelot_bootstrap::execute_shell(&bootstrap_config, &shell_config)?;
        ocelot_bootstrap::shutdown()?;
    } else if let Some(orchestrator) = config.to_orchestrator_config() {
        // Supervise mode (normal operation)
        init_tracing_subscriber(config.log_level);
        ocelot_bootstrap::execute_supervise(&bootstrap_config, orchestrator)?;
        ocelot_bootstrap::shutdown()?;
    } else {
        return Err(Error::InvalidConfig {
            message: "Configuration must specify either shell mode or supervise mode".to_string(),
        });
    }

    Ok(0)
}

/// Initializes the tracing subscriber with the specified log level.
fn init_tracing_subscriber(log_level: tracing::Level) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level.as_str())),
        )
        .init();
}
