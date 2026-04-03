use std::os::unix::fs::symlink;

use crate::config::SymlinkSpec;

/// Creates symlinks as specified in the configuration.
///
/// For each symlink spec, creates the parent directory if needed and then
/// creates the symlink. Warnings are logged if the target does not exist.
pub fn create_symlinks(specs: &[SymlinkSpec]) -> Result<(), nix::Error> {
    for spec in specs {
        create_symlink(spec)?;
    }
    Ok(())
}

/// Creates a single symlink, ensuring the parent directory exists.
///
/// Logs a warning if the symlink target does not exist, but still creates
/// the symlink.
pub fn create_symlink(SymlinkSpec { source, target }: &SymlinkSpec) -> Result<(), nix::Error> {
    if let Some(parent) = std::path::Path::new(&target).parent()
        && let Some(parent_str) = parent.to_str()
        && !parent_str.is_empty()
    {
        ensure_dir_all(parent_str)?;
    }

    if !std::path::Path::new(&source).exists() {
        tracing::warn!("Symlink target '{}' does not exist, creating symlink anyway", source);
    }

    symlink(source, target).map_err(|_| nix::Error::EIO)?;

    tracing::info!("Created symlink {} -> {}", target, source);
    Ok(())
}

/// Recursively creates a directory and all parent directories.
fn ensure_dir_all(path: &str) -> Result<(), nix::Error> {
    std::fs::create_dir_all(path).map_err(|_| nix::Error::EIO)
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
        fs::create_dir_all(&temp_dir).unwrap();

        let target = temp_dir.join("target_file");
        let link = temp_dir.join("link_file");

        fs::write(&target, "test").unwrap();

        let spec = SymlinkSpec {
            source: target.to_str().unwrap().to_string(),
            target: link.to_str().unwrap().to_string(),
        };

        create_symlink(&spec).unwrap();
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());

        drop(fs::remove_dir_all(&temp_dir));
    }

    #[test]
    fn test_create_symlinks_empty_list() { create_symlinks(&[]).unwrap(); }
}
