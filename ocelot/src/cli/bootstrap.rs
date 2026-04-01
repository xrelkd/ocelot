//! Bootstrap subcommand handler.
//!
//! This module handles the `ocelot bootstrap` (alias: `boot`) subcommand,
//! which acts as an initramfs init system for QEMU VMs.

use std::path::Path;

use crate::{config::BootstrapConfig, error::Error};

/// Runs the bootstrap subcommand.
///
/// Loads the bootstrap configuration file, converts it to the appropriate
/// types, and executes the bootstrap flow.
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
    let orchestrator = config.to_orchestrator_config();
    ocelot_bootstrap::execute(&bootstrap_config, orchestrator)?;
    Ok(0)
}
