use std::path::Path;

use crate::{config::MountSpec, error::Error, mount};

pub fn pre(specs: &[MountSpec]) -> Result<(), Error> {
    let root = Path::new("/");
    for spec in specs {
        if spec.target == root {
            continue;
        }
        let target = mount::mount(spec)?;
        tracing::debug!(
            "Pre-switch mounted {} at {} with flags: {:?}",
            spec.fstype,
            target.display(),
            spec.flags
        );
    }
    Ok(())
}

pub fn post(specs: &[MountSpec]) -> Result<(), Error> {
    let root = Path::new("/");
    for spec in specs {
        if spec.target == root {
            continue;
        }
        let target = mount::mount(spec)?;

        tracing::debug!(
            "Post-switch mounted {} at {} with flags: {:?}",
            spec.fstype,
            target.display(),
            spec.flags
        );
    }
    Ok(())
}
