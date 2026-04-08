/// Hook execution phase functions.
///
/// These functions execute custom commands at specific points in the bootstrap
/// process.
use std::process::Command;

use snafu::ResultExt;

use crate::{
    config::{HookFailurePolicy, HookSpec},
    error,
    error::Error,
};

/// Executes hook commands before `switch_root`.
///
/// Runs each hook in sequence. The behavior on failure depends on the hook's
/// [`HookFailurePolicy`]: `Warn` logs a warning, `Abort` returns an error,
/// and `Retry` attempts the hook one more time.
pub fn pre(specs: &[HookSpec]) -> Result<(), Error> {
    for spec @ HookSpec { name, .. } in specs {
        invoke_hook(spec)?;
        tracing::info!("Pre-switch: executed hook '{name}'");
    }
    Ok(())
}

/// Executes hook commands after `switch_root`.
///
/// Currently a placeholder - post-switch hooks are not yet implemented.
pub fn post(specs: &[HookSpec]) -> Result<(), Error> {
    for spec @ HookSpec { name, .. } in specs {
        invoke_hook(spec)?;
        tracing::info!("Post-switch: executed hook '{name}'");
    }
    Ok(())
}

fn invoke_hook(
    HookSpec { name, command, arguments, on_failure, .. }: &HookSpec,
) -> Result<(), Error> {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .with_context(|_| error::SpawnHookSnafu { name: name.clone() })?;

    if output.status.success() {
        return Ok(());
    }

    match on_failure {
        HookFailurePolicy::Warn => {
            tracing::warn!(
                "Pre-switch hook '{name}' failed (exit code: {}): {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            );
            Ok(())
        }
        HookFailurePolicy::Abort => Err(Error::ExecuteHook {
            name: name.clone(),
            error: String::from_utf8_lossy(&output.stderr).to_string(),
        }),
        HookFailurePolicy::Retry => {
            let output = Command::new(command)
                .args(arguments)
                .output()
                .with_context(|_| error::SpawnHookSnafu { name: name.clone() })?;
            if output.status.success() {
                Ok(())
            } else {
                Err(Error::ExecuteHook {
                    name: name.clone(),
                    error: String::from_utf8_lossy(&output.stderr).to_string(),
                })
            }
        }
    }
}
