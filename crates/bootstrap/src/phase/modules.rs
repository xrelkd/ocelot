/// Kernel module loading phase functions.
///
/// These functions load kernel modules during the bootstrap process, supporting
/// both list mode (specific modules by name) and scan mode (discover all
/// modules in a directory).
use std::{
    ffi::CString,
    fs::File,
    io::{BufReader, Seek, SeekFrom, Write},
    os::fd::{AsRawFd, FromRawFd},
    path::Path,
};

use nix::{kmod, kmod::ModuleInitFlags, sys::memfd::memfd_create};
use snafu::ResultExt;

use crate::{config::ModulesConfig, error, error::Error};

/// Loads kernel modules before `switch_root`.
///
/// Supports both list mode (load specific modules by name) and scan mode
/// (discover and load all modules in a directory).
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn pre(config: &ModulesConfig) -> Result<(), Error> {
    load_modules(config);
    tracing::debug!("Pre-switch: loaded kernel modules");
    Ok(())
}

/// Loads kernel modules after `switch_root`.
///
/// Currently a placeholder - post-switch module loading is not yet implemented.
#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(config: &ModulesConfig) -> Result<(), Error> {
    load_modules(config);
    tracing::debug!("Post-switch: loaded kernel modules");
    Ok(())
}

/// Loads kernel modules based on the configuration.
///
/// Dispatches to list mode or scan mode based on `ModulesConfig` variant.
/// Errors are logged as warnings and do not stop the boot process.
fn load_modules(ModulesConfig { directory, module_file_names }: &ModulesConfig) {
    for name in module_file_names {
        tracing::info!("Loading kernel module: {name}");
        let path = directory.join(name);
        match load_module_from_path(&path) {
            Ok(()) => tracing::info!("Successfully loaded module: {}", path.display()),
            Err(source) => {
                tracing::warn!("Failed to load module: {}, error: {source}", path.display());
            }
        }
    }
}

#[derive(Clone, Copy)]
enum DecompressFormat {
    Xz,
    Gz,
}

fn decompress_module<W: Write>(
    data: &[u8],
    format: DecompressFormat,
    writer: &mut W,
) -> Result<(), std::io::Error> {
    match format {
        DecompressFormat::Xz => {
            let mut reader = BufReader::new(data);
            lzma_rs::xz_decompress(&mut reader, writer).map_err(|error| match error {
                lzma_rs::error::Error::IoError(error) => error,
                error => std::io::Error::other(error),
            })?;
            Ok(())
        }
        DecompressFormat::Gz => {
            let mut gz_decoder = flate2::read::GzDecoder::new(data);
            let _ = std::io::copy(&mut gz_decoder, writer)?;
            Ok(())
        }
    }
}

#[inline]
fn load_compressed_module(path: impl AsRef<Path>, format: DecompressFormat) -> Result<(), Error> {
    let path = path.as_ref();
    let compressed = std::fs::read(path)
        .with_context(|_| error::OpenModuleSnafu { path: path.to_path_buf() })?;

    let fd = {
        let fd = {
            let memfd_name = {
                let file_name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
                format!("kmod-{file_name}")
            };
            memfd_create(memfd_name.as_str(), nix::sys::memfd::MFdFlags::empty())
                .map_err(std::io::Error::from)
                .with_context(|_| error::CreateMemfdSnafu { path: path.to_path_buf() })?
        };

        // NOTE: We create a File from the raw fd to write data, then forget it so the
        // OwnedFd retains ownership and closes the fd when dropped.
        #[expect(unsafe_code, reason = "memfd_create returns a valid owned fd")]
        let mut file = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
        decompress_module(&compressed, format, &mut file)
            .with_context(|_| error::DecompressModuleSnafu { path: path.to_path_buf() })?;
        let _ = file
            .seek(SeekFrom::Start(0))
            .with_context(|_| error::CreateMemfdSnafu { path: path.to_path_buf() })?;
        std::mem::forget(file);

        fd
    };

    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&fd, &params, ModuleInitFlags::empty())
        .with_context(|_| error::InitializeModuleSnafu { path: path.to_path_buf() })?;
    Ok(())
}

#[inline]
fn load_uncompressed_module(path: impl AsRef<Path>) -> Result<(), Error> {
    let path = path.as_ref();
    let file =
        File::open(path).with_context(|_| error::OpenModuleSnafu { path: path.to_path_buf() })?;
    let params = CString::new("").expect("empty string is valid CStr");
    kmod::finit_module(&file, &params, ModuleInitFlags::empty())
        .with_context(|_| error::InitializeModuleSnafu { path: path.to_path_buf() })?;
    Ok(())
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
