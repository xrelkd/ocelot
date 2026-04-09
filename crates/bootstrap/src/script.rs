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

#[cfg(test)]
mod tests {
    use super::execute_boot_script;
    use crate::config::{BootScriptConfig, OnFailurePolicy};

    #[test]
    fn test_boot_script_success() {
        let config = BootScriptConfig {
            command: "true".to_string(),
            arguments: Vec::new(),
            on_failure: OnFailurePolicy::Warn,
            working_directory: None,
        };
        assert!(execute_boot_script(&config).is_ok());
    }

    #[test]
    fn test_boot_script_failure_warn_policy() {
        let config = BootScriptConfig {
            command: "false".to_string(),
            arguments: Vec::new(),
            on_failure: OnFailurePolicy::Warn,
            working_directory: None,
        };
        assert!(execute_boot_script(&config).is_ok());
    }

    #[test]
    fn test_boot_script_failure_abort_policy() {
        let config = BootScriptConfig {
            command: "false".to_string(),
            arguments: Vec::new(),
            on_failure: OnFailurePolicy::Abort,
            working_directory: None,
        };
        assert!(execute_boot_script(&config).is_err());
    }
}
