//! Bootstrap subcommand handler.
//!
//! This module handles the `ocelot bootstrap` (alias: `boot`) subcommand,
//! which acts as an initramfs init system for QEMU VMs.

use std::path::Path;

use crate::{config::BootstrapConfig, error::Error};

/// Runs the bootstrap subcommand.
///
/// Loads the bootstrap configuration file, converts it to the appropriate
/// types, and executes the bootstrap flow in either shell or supervise mode.
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

    if let Some(shell_config) = config.to_shell_config() {
        // Shell mode for debugging
        ocelot_bootstrap::execute_shell(&bootstrap_config, &shell_config)?;
        ocelot_bootstrap::shutdown()?;
    } else if let Some(orchestrator) = config.to_orchestrator_config() {
        // Supervise mode (normal operation)
        ocelot_bootstrap::execute_supervise(&bootstrap_config, orchestrator)?;
        ocelot_bootstrap::shutdown()?;
    } else {
        return Err(Error::InvalidConfig {
            message: "Configuration must specify either shell mode or supervise mode".to_string(),
        });
    }

    Ok(0)
}
