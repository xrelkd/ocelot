use std::process::Command;

use crate::{
    config::{HookSpec, MountFailurePolicy},
    error::Error,
};

pub fn pre(specs: &[HookSpec]) -> Result<(), Error> {
    for spec in specs {
        let output = Command::new(&spec.command).args(&spec.arguments).output().map_err(|e| {
            Error::Hook { message: format!("Failed to execute hook '{}': {}", spec.name, e) }
        })?;

        if output.status.success() {
            continue;
        }

        match spec.on_failure {
            MountFailurePolicy::Warn => {
                tracing::warn!(
                    "Pre-switch hook '{}' failed (exit code: {}): {}",
                    spec.name,
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            MountFailurePolicy::Abort => {
                return Err(Error::Hook {
                    message: format!(
                        "Hook '{}' failed: {}",
                        spec.name,
                        String::from_utf8_lossy(&output.stderr)
                    ),
                });
            }
            MountFailurePolicy::Retry => {
                let output2 =
                    Command::new(&spec.command).args(&spec.arguments).output().map_err(|e| {
                        Error::Hook {
                            message: format!(
                                "Failed to execute hook '{}' (retry): {}",
                                spec.name, e
                            ),
                        }
                    })?;

                if !output2.status.success() {
                    return Err(Error::Hook {
                        message: format!(
                            "Hook '{}' failed after retry: {}",
                            spec.name,
                            String::from_utf8_lossy(&output2.stderr)
                        ),
                    });
                }
            }
        }

        tracing::info!("Pre-switch: executed hook '{}'", spec.name);
    }
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_specs: &[HookSpec]) -> Result<(), Error> {
    tracing::debug!("Post-switch: hooks (not implemented)");
    Ok(())
}
