use std::{ffi::CString, fs::File};

use nix::{kmod, kmod::ModuleInitFlags};

use crate::config::ModuleConfig;

/// Loads all kernel modules specified in the configuration.
/// Errors are logged as warnings and do not stop the boot process.
pub fn load_modules(config: &ModuleConfig) {
    let dir = config.dir.as_deref().unwrap_or("/lib/modules");

    for name in &config.list {
        let path = format!("{dir}/{name}.ko");
        tracing::info!("Loading kernel module: {name}");

        match load_module_from_path(&path) {
            Ok(()) => {
                tracing::info!("Successfully loaded module: {name}");
            }
            Err(source) => {
                tracing::warn!("Failed to load module {name}: {source}");
            }
        }
    }
}

fn load_module_from_path(path: &str) -> Result<(), nix::Error> {
    let file = File::open(path).map_err(|_| nix::Error::ENOENT)?;
    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&file, &params, ModuleInitFlags::empty())
}

#[cfg(test)]
mod tests {
    use super::load_modules;
    use crate::config::ModuleConfig;

    #[test]
    fn test_load_modules_empty_list() {
        let config = ModuleConfig { dir: Some("/lib/modules".to_string()), list: vec![] };
        load_modules(&config);
    }
}
