use std::{path::Path, time::Duration};

use crate::{
    Error,
    config::{BootScriptConfig, OnFailurePolicy},
};

/// Executes the configured boot script.
///
/// Uses `ocelot_entry::execute` for zombie reaping, signal forwarding, and
/// timeout support. The script runs with inherited environment and optional
/// working directory.
///
/// # Errors
///
/// Returns an error if execution fails and `on_failure` is `Abort`.
/// Logs a warning and returns `Ok(())` if `on_failure` is `Warn`.
pub fn execute_boot_script(
    BootScriptConfig { command, arguments, on_failure, working_directory }: &BootScriptConfig,
) -> Result<(), Error> {
    if let Some(working_dir) = working_directory {
        tracing::info!("Setting working directory for boot script: {working_dir}");
        if let Err(source) = std::env::set_current_dir(working_dir) {
            tracing::warn!("Failed to set working directory for boot script: {source}");
        }
    }

    tracing::info!("Executing boot script: {command} {}", arguments.join(" "));

    let timeout = Duration::from_secs(300);
    let result = ocelot_entry::execute(command, arguments.clone(), Some(timeout));

    match result {
        Ok(0) => {
            tracing::info!("Boot script completed successfully");
            if let Err(source) = std::env::set_current_dir(Path::new("/")) {
                tracing::warn!("Failed to set working directory to \"/\", error: {source}");
            }
            Ok(())
        }
        Ok(exit_code) => {
            tracing::warn!("Boot script exited with non-zero code: {exit_code}");
            match on_failure {
                OnFailurePolicy::Warn => Ok(()),
                OnFailurePolicy::Abort => {
                    Err(Error::ExecuteBootScript { source: ocelot_entry::Error::ExecuteChild })
                }
            }
        }
        Err(source) => {
            tracing::error!("Failed to execute boot script: {source}");
            match on_failure {
                OnFailurePolicy::Warn => Ok(()),
                OnFailurePolicy::Abort => Err(Error::ExecuteBootScript { source }),
            }
        }
    }
}
