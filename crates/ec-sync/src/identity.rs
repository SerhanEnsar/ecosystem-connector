//! What a file looks like on disk, read without opening it.

use std::fs::Metadata;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::{FileIdentity, Seen};

impl FileIdentity {
    /// The identity of the file `meta` describes, on platforms that have one.
    pub fn of(meta: &Metadata) -> Option<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Some(Self {
                dev: meta.dev(),
                ino: meta.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            let _ = meta;
            None
        }
    }
}

impl Seen {
    /// The size and modification time `meta` describes.
    pub fn of(meta: &Metadata) -> Self {
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as i128);
        Self {
            size: meta.len(),
            mtime_ns,
        }
    }
}

/// The size, modification time and identity of the regular file at `path`, or `None` if
/// there is none (missing, a folder, or a link, which are never followed).
pub fn stat(path: &Path) -> Option<(Seen, Option<FileIdentity>)> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    meta.is_file()
        .then(|| (Seen::of(&meta), FileIdentity::of(&meta)))
}
