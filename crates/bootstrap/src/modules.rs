use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    path::Path,
};

use nix::{kmod, kmod::ModuleInitFlags, sys::memfd::memfd_create};
use snafu::ResultExt;

use crate::{config::ModulesConfig, error, error::Error};

/// Loads kernel modules based on the configuration.
///
/// Dispatches to list mode or scan mode based on `ModulesConfig` variant.
/// Errors are logged as warnings and do not stop the boot process.
pub fn load_modules(config: &ModulesConfig) {
    match config {
        ModulesConfig::List { dir, names } => {
            let dir = dir.as_deref().unwrap_or("/lib/modules");
            for name in names {
                #[expect(
                    clippy::case_sensitive_file_extension_comparisons,
                    reason = "Kernel module extensions are case-sensitive by convention (.ko, \
                              .ko.xz, .ko.gz)"
                )]
                let path = if name.ends_with(".ko")
                    || name.ends_with(".ko.xz")
                    || name.ends_with(".ko.gz")
                {
                    format!("{dir}/{name}")
                } else {
                    format!("{dir}/{name}.ko")
                };
                tracing::info!("Loading kernel module: {name}");

                match load_module_from_path(&path) {
                    Ok(()) => tracing::info!("Successfully loaded module: {name}"),
                    Err(source) => tracing::warn!("Failed to load module {name}, error: {source}"),
                }
            }
        }
        ModulesConfig::Scan { dir, names } => {
            if let Some(filtered_names) = names {
                let dir = dir.as_str();
                for name in filtered_names {
                    let path = format!("{dir}/{name}");
                    tracing::info!("Loading kernel module: {name}");

                    match load_module_from_path(&path) {
                        Ok(()) => tracing::info!("Successfully loaded module: {name}"),
                        Err(source) => {
                            tracing::warn!("Failed to load module {name}, error: {source}");
                        }
                    }
                }
            } else {
                scan_and_load_modules(dir);
            }
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

        match load_module_from_path(&path) {
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

#[derive(Clone, Copy)]
enum DecompressFormat {
    Xz,
    Gz,
}

fn decompress_module(data: &[u8], format: DecompressFormat) -> Result<Vec<u8>, std::io::Error> {
    match format {
        DecompressFormat::Xz => {
            let mut decoded = Vec::new();
            let mut reader = BufReader::new(data);
            lzma_rs::xz_decompress(&mut reader, &mut decoded).map_err(|e| match e {
                lzma_rs::error::Error::IoError(e) => e,
                e => std::io::Error::other(e),
            })?;
            Ok(decoded)
        }
        DecompressFormat::Gz => {
            let mut gz_decoder = flate2::read::GzDecoder::new(data);
            let mut decoded = Vec::new();
            let _ = gz_decoder.read_to_end(&mut decoded)?;
            Ok(decoded)
        }
    }
}

fn write_module_to_memfd(data: &[u8], name: &str) -> Result<OwnedFd, std::io::Error> {
    let fd = memfd_create(name, nix::sys::memfd::MFdFlags::empty())?;

    // SAFETY: memfd_create returns a valid owned file descriptor. We create a
    // File from the raw fd to write data, then forget it so the OwnedFd retains
    // ownership and closes the fd when dropped.
    #[expect(unsafe_code, reason = "memfd_create returns a valid owned fd")]
    let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
    file.write_all(data)?;
    file.seek(SeekFrom::Start(0)).map(|_| ())?;
    std::mem::forget(file);
    Ok(fd)
}

#[inline]
fn load_compressed_module(path: impl AsRef<Path>, format: DecompressFormat) -> Result<(), Error> {
    let path = path.as_ref();
    let compressed = std::fs::read(path)
        .with_context(|_| error::OpenModuleSnafu { path: path.to_path_buf() })?;

    let decompressed = decompress_module(&compressed, format)
        .with_context(|_| error::DecompressModuleSnafu { path: path.to_path_buf() })?;

    let memfd_name = format!("kmod-{}\0", path.display());
    let fd = write_module_to_memfd(&decompressed, &memfd_name)
        .with_context(|_| error::CreateMemfdSnafu { path: path.to_path_buf() })?;

    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&fd, &params, ModuleInitFlags::empty()).with_context(|_| error::MountSnafu {
        operation: format!("kernel module '{}'", path.display()),
    })
}

#[inline]
fn load_uncompressed_module(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    let file =
        File::open(path).with_context(|_| error::OpenModuleSnafu { path: path.to_path_buf() })?;
    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&file, &params, ModuleInitFlags::empty()).with_context(|_| {
        error::MountSnafu { operation: format!("kernel module '{}'", path.display()) }
    })
}

fn load_module_from_path(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    if path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("xz")
            && path.file_stem().is_some_and(|s| {
                Path::new(s).extension().is_some_and(|e| e.eq_ignore_ascii_case("ko"))
            })
    }) {
        load_compressed_module(path, DecompressFormat::Xz)
    } else if path.extension().is_some_and(|ext| {
        ext.eq_ignore_ascii_case("gz")
            && path.file_stem().is_some_and(|s| {
                Path::new(s).extension().is_some_and(|e| e.eq_ignore_ascii_case("ko"))
            })
    }) {
        load_compressed_module(path, DecompressFormat::Gz)
    } else {
        load_uncompressed_module(path)
    }
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
        let config = ModulesConfig::Scan { dir: "/nonexistent/modules".to_string(), names: None };
        load_modules(&config);
    }
}
