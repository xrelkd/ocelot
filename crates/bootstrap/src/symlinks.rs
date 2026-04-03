use std::{os::unix::fs::symlink, path::Path};

use snafu::ResultExt;

use crate::{
    config::SymlinkSpec,
    error::{self, Error},
};

/// Creates symlinks as specified in the configuration.
///
/// For each symlink spec, creates the parent directory if needed and then
/// creates the symlink. Warnings are logged if the target does not exist.
///
/// # Errors
///
/// Returns an error if directory creation or symlink creation fails.
pub fn create_symlinks(specs: &[SymlinkSpec]) -> Result<(), Error> {
    for spec in specs {
        create_symlink(spec)?;
    }
    Ok(())
}

/// Creates a single symlink, ensuring the parent directory exists.
///
/// Logs a warning if the symlink target does not exist, but still creates
/// the symlink.
///
/// # Errors
///
/// Returns an error if the parent directory or symlink cannot be created.
pub fn create_symlink(SymlinkSpec { source, target }: &SymlinkSpec) -> Result<(), Error> {
    if let Some(parent) = Path::new(&target).parent()
        && let Some(parent_str) = parent.to_str()
        && !parent_str.is_empty()
    {
        ensure_dir_all(parent_str)?;
    }

    if !Path::new(&source).exists() {
        tracing::warn!("Symlink target '{source}' does not exist, creating symlink anyway");
    }

    symlink(source, target).with_context(|_| error::CreateSymlinkSnafu {
        link_source: source.clone(),
        target: target.clone(),
    })?;

    tracing::info!("Created symlink {target} -> {source}");
    Ok(())
}

/// Recursively creates a directory and all parent directories.
fn ensure_dir_all(path: &str) -> Result<(), Error> {
    std::fs::create_dir_all(path)
        .with_context(|_| error::CreateDirectorySnafu { path: path.to_string() })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{create_symlink, create_symlinks};
    use crate::config::SymlinkSpec;

    #[test]
    fn test_create_symlink_in_temp_dir() {
        let temp_dir = std::env::temp_dir().join("ocelot_test_symlinks");
        drop(fs::remove_dir_all(&temp_dir));
        fs::create_dir_all(&temp_dir).expect("temp dir creation should succeed");

        let target = temp_dir.join("target_file");
        let link = temp_dir.join("link_file");

        fs::write(&target, "test").expect("write test file should succeed");

        let spec = SymlinkSpec {
            source: target.to_str().expect("target path should be valid utf-8").to_string(),
            target: link.to_str().expect("link path should be valid utf-8").to_string(),
        };

        create_symlink(&spec).expect("symlink creation should succeed");
        assert!(
            link.symlink_metadata()
                .expect("symlink metadata should exist")
                .file_type()
                .is_symlink()
        );

        drop(fs::remove_dir_all(&temp_dir));
    }

    #[test]
    fn test_create_symlinks_empty_list() {
        create_symlinks(&[]).expect("empty symlink list should succeed");
    }
}
