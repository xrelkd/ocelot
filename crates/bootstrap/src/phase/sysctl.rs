use std::{fs, io::Write, path::Path};

use snafu::ResultExt;

use crate::{
    config::Sysctl,
    error::{CreateDirectorySnafu, Error},
};

pub fn pre(Sysctl { key_values }: &Sysctl) -> Result<(), Error> {
    for (key, value) in key_values {
        let sysctl_path = format!("/proc/sys/{}", key.replace('.', "/"));
        let path = Path::new(&sysctl_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|_| CreateDirectorySnafu { path: parent.to_path_buf() })?;
        }

        // FIXME: AI: Use ResultExt::with_context
        let mut file = fs::File::create(path).map_err(|e| Error::Mount {
            operation: format!("write sysctl {key}"),
            source: nix::Error::from_raw(e.raw_os_error().unwrap_or(1)),
        })?;

        // FIXME: AI: Use ResultExt::with_context
        file.write_all(value.as_bytes()).map_err(|e| Error::Mount {
            operation: format!("write sysctl {key}"),
            source: nix::Error::from_raw(e.raw_os_error().unwrap_or(1)),
        })?;

        tracing::debug!("Pre-switch: set sysctl {key} = {value}");
    }
    Ok(())
}

#[expect(clippy::unnecessary_wraps, reason = "Phase function may return errors in future")]
pub fn post(_config: &Sysctl) -> Result<(), Error> {
    tracing::debug!("Post-switch: sysctl (not implemented)");
    Ok(())
}
