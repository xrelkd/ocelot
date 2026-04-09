//! Atime (access time) update modes for mount specifications.
//!
//! Atime controls when the last access time of files is updated. Different
//! modes offer trade-offs between performance and POSIX compliance.

use serde::Deserialize;

/// Atime update mode for mounted filesystems.
///
/// The default behavior is relatime (relative atime), which is POSIX-compliant
/// and provides good performance for most workloads.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AtimeMode {
    /// Default atime behavior (typically relatime on Linux).
    #[default]
    Default,
    /// Never update access times (noatime). Best performance, breaks some
    /// legacy applications that rely on atime.
    NoAtime,
    /// Update atime only if current atime is older than mtime or ctime
    /// (relatime). This is the Linux default and provides a good balance.
    RelAtime,
    /// Always update atime on every access (strictatime). POSIX compliant but
    /// can cause significant performance issues on busy systems.
    StrictAtime,
    /// Lazily update atime (lazytime). Very high performance; atime updates
    /// happen in memory and are flushed periodically. Mount must be remounted
    /// for changes to be visible.
    LazyTime,
}
