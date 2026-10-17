//! Small helpers for storing private files safely.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::{Error, Result};

/// Writes `data` to `path` atomically: a temporary file is written and synced first,
/// then renamed over the target, so a crash never leaves a half-written file.
///
/// On Unix the file is created with mode `0600` because it may hold a private key.
/// On Windows the file inherits the ACL of the user's profile directory.
pub(crate) fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    let io = |action, source| Error::Io {
        action,
        path: path.to_path_buf(),
        source,
    };

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&tmp).map_err(|e| io("create", e))?;
    file.write_all(data).map_err(|e| io("write", e))?;
    file.sync_all().map_err(|e| io("sync", e))?;
    drop(file);
    fs::rename(&tmp, path).map_err(|e| io("rename", e))?;

    // Make the rename itself durable.
    if let Some(dir) = path.parent()
        && let Ok(dir) = File::open(dir)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

/// Creates `dir` and its parents. On Unix the directory is private to the user.
pub(crate) fn create_private_dir(dir: &Path) -> Result<()> {
    let io = |source| Error::Io {
        action: "create directory",
        path: dir.to_path_buf(),
        source,
    };
    fs::create_dir_all(dir).map_err(io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).map_err(io)?;
    }
    Ok(())
}

/// Reads `path`, returning `Ok(None)` when it does not exist.
pub(crate) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(data) => Ok(Some(data)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            action: "read",
            path: path.to_path_buf(),
            source,
        }),
    }
}
