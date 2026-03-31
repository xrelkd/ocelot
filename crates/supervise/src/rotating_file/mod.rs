#[cfg(test)]
mod tests;

use std::{
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{fs::File, io, io::AsyncWrite};

use crate::supervisor::LogRotationConfig;

/// A file writer that automatically rotates log files based on size and/or time
/// constraints.
///
/// Rotation occurs before writing data that would exceed configured limits.
/// Rotated files are renamed with a timestamp suffix and old files are deleted
/// to maintain the maximum file count.
///
/// # Examples
///
/// ```
/// use ocelot_supervise::rotating_file::RotatingFile;
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let rotation_config = /* config from LogRotationConfig */;
/// let mut rotating_file = RotatingFile::new(PathBuf::from("app.log"), rotation_config).await?;
/// rotating_file.write_all(b"Hello world!").await?;
/// # Ok(())
/// # }
/// ```
pub struct RotatingFile {
    base_path: PathBuf,
    current_file: Option<File>,
    rotation: LogRotationConfig,
    current_size: u64,
    last_rotation: SystemTime,
}

impl RotatingFile {
    /// Creates a new `RotatingFile` with the given base path and rotation
    /// configuration.
    ///
    /// Opens the file in append mode (creating it if it doesn't exist) and
    /// initializes rotation tracking based on the file's current size and
    /// modification time.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or metadata cannot be
    /// read.
    pub async fn new(base_path: PathBuf, rotation: LogRotationConfig) -> io::Result<Self> {
        let mut opts_base = tokio::fs::OpenOptions::new();
        let file = if let Some(mode) = rotation.mode {
            opts_base.append(true).create(true).mode(mode).open(&base_path).await?
        } else {
            opts_base.append(true).create(true).open(&base_path).await?
        };
        let metadata = file.metadata().await?;
        let current_size = metadata.len();
        let last_rotation = metadata.modified().unwrap_or_else(|_| SystemTime::now());

        let this =
            Self { base_path, current_file: Some(file), rotation, current_size, last_rotation };

        if this.rotation.max_age_days.is_some() {
            this.cleanup_old_files().await?;
        }

        Ok(this)
    }

    async fn cleanup_old_files(&self) -> io::Result<()> {
        let Some(parent) = self.base_path.parent() else {
            return Ok(());
        };
        let Some(max_age_days) = self.rotation.max_age_days else {
            return Ok(());
        };
        let now = SystemTime::now();
        let mut entries = tokio::fs::read_dir(parent).await?;
        let base_name = self.base_path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with(base_name)
                && name.contains('.')
                && let Ok(metadata) = entry.metadata().await
                && let Ok(mod_time) = metadata.modified()
            {
                let age_days =
                    now.duration_since(mod_time).map(|d| d.as_secs() / 86400).unwrap_or(0);
                if age_days >= u64::from(max_age_days) {
                    let _unused = tokio::fs::remove_file(&path).await;
                }
            }
        }
        Ok(())
    }

    #[inline]
    fn needs_rotation(&self, incoming_len: usize) -> bool {
        let size_trigger = self
            .rotation
            .max_size_bytes
            .is_some_and(|max| self.current_size + incoming_len as u64 > max);
        let time_trigger = self.rotation.rotation_interval_secs.is_some_and(|interval| {
            SystemTime::now()
                .duration_since(self.last_rotation)
                .map(|d| d.as_secs() > interval)
                .unwrap_or(true)
        });
        size_trigger || time_trigger
    }

    fn perform_rotation_sync(&mut self) -> io::Result<()> {
        {
            let old_tokio_file = self
                .current_file
                .take()
                .ok_or_else(|| io::Error::other("RotatingFile: no current file to rotate"))?;
            let old_file = old_tokio_file.into_std();
            drop(old_file);
        }

        {
            let timestamp =
                SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let rotated_path = format!("{}.{}", self.base_path.display(), timestamp);
            std::fs::rename(&self.base_path, &rotated_path)?;
        }

        if let (Some(max_files), Some(parent)) = (self.rotation.max_files, self.base_path.parent())
        {
            let entries = std::fs::read_dir(parent)?;
            let max_files = max_files as usize;
            let mut rotated_files = Vec::new();
            let now = SystemTime::now();
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with(
                        self.base_path.file_name().and_then(|n| n.to_str()).unwrap_or_default(),
                    )
                    && name.contains('.')
                    && let Ok(metadata) = entry.metadata()
                    && let Ok(mod_time) = metadata.modified()
                {
                    let age_days =
                        now.duration_since(mod_time).map(|d| d.as_secs() / 86400).unwrap_or(0);

                    if self.rotation.max_age_days.is_some_and(|max| age_days > u64::from(max)) {
                        let _unused = std::fs::remove_file(&path);
                        continue;
                    }

                    rotated_files.push((mod_time, path));
                }
            }
            rotated_files.sort_by_key(|(time, _path)| *time);
            if rotated_files.len() > max_files {
                let excess = rotated_files.len() - max_files;
                for (_, path) in rotated_files.iter().take(excess) {
                    let _unused = std::fs::remove_file(path);
                }
            }
        }

        self.current_file = {
            let new_std_file = if let Some(mode) = self.rotation.mode {
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .mode(mode)
                    .open(&self.base_path)?
            } else {
                std::fs::OpenOptions::new().append(true).create(true).open(&self.base_path)?
            };
            Some(File::from_std(new_std_file))
        };
        self.current_size = 0;
        self.last_rotation = SystemTime::now();

        Ok(())
    }
}

impl AsyncWrite for RotatingFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.as_mut().get_mut();

        if this.needs_rotation(buf.len())
            && let Err(e) = this.perform_rotation_sync()
        {
            return Poll::Ready(Err(e));
        }

        let Some(file) = &mut this.current_file else {
            return Poll::Ready(Err(io::Error::other(
                "RotatingFile: current file is None after rotation",
            )));
        };

        let result = Pin::new(file).poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = result {
            this.current_size += n as u64;
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.current_file.as_mut().map_or(Poll::Ready(Ok(())), |file| {
            let pin = Pin::new(file);
            pin.poll_flush(cx)
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        this.current_file.as_mut().map_or(Poll::Ready(Ok(())), |file| {
            let pin = Pin::new(file);
            pin.poll_shutdown(cx)
        })
    }
}
