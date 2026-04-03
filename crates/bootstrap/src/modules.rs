use std::{ffi::CString, fs::File, path::Path};

use nix::{kmod, kmod::ModuleInitFlags};

use crate::config::ModulesConfig;

/// Loads kernel modules based on the configuration.
///
/// Dispatches to list mode or scan mode based on `ModulesConfig` variant.
/// Errors are logged as warnings and do not stop the boot process.
pub fn load_modules(config: &ModulesConfig) {
    match config {
        ModulesConfig::List { dir, names } => {
            let dir = dir.as_deref().unwrap_or("/lib/modules");
            for name in names {
                let path = format!("{dir}/{name}.ko");
                tracing::info!("Loading kernel module: {name}");

                match load_module_from_path(&path) {
                    Ok(()) => tracing::info!("Successfully loaded module: {name}"),
                    Err(source) => tracing::warn!("Failed to load module {name}, error: {source}"),
                }
            }
        }
        ModulesConfig::Scan { dir } => {
            scan_and_load_modules(dir);
        }
    }
}

/// Scans a directory for kernel module files and loads each one.
///
/// Loads `.ko`, `.ko.xz`, and `.ko.gz` files. Errors are logged as warnings.
fn scan_and_load_modules(dir: &str) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) => {
            tracing::info!("Module directory {dir} not found, skipping module loading: {source}");
            return;
        }
    };

    let mut loaded = 0;
    let mut failed = 0;
    let mut total = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let is_module = {
            let path = Path::new(file_name);
            path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("ko"))
                || path
                    .file_stem()
                    .and_then(|stem| Path::new(stem).extension())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("ko"))
        };

        if !is_module {
            continue;
        }

        total += 1;
        tracing::info!("Loading kernel module: {file_name}");

        match load_module_from_path(path.to_str().unwrap()) {
            Ok(()) => {
                tracing::info!("Successfully loaded module: {file_name}");
                loaded += 1;
            }
            Err(source) => {
                tracing::warn!("Failed to load module {file_name}, error: {source}");
                failed += 1;
            }
        }
    }

    tracing::info!("Module scan complete: {loaded} loaded, {failed} failed, {total} total");
}

fn load_module_from_path(path: &str) -> Result<(), nix::Error> {
    let file = File::open(path).map_err(|_| nix::Error::ENOENT)?;
    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&file, &params, ModuleInitFlags::empty())
}

#[cfg(test)]
mod tests {
    use super::load_modules;
    use crate::config::ModulesConfig;

    #[test]
    fn test_load_modules_empty_list() {
        let config =
            ModulesConfig::List { dir: Some("/lib/modules".to_string()), names: Vec::new() };
        load_modules(&config);
    }

    #[test]
    fn test_load_modules_default_dir() {
        let config = ModulesConfig::List { dir: None, names: Vec::new() };
        load_modules(&config);
    }

    #[test]
    fn test_scan_mode_empty_dir() {
        let config = ModulesConfig::Scan { dir: "/nonexistent/modules".to_string() };
        load_modules(&config);
    }
}
