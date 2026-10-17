use std::path::{Path, PathBuf};

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Network or connection error.
    #[error(transparent)]
    Net(#[from] ec_net::Error),
    /// Reading or writing a local file failed.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was being done.
        action: &'static str,
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A received path was refused.
    #[error("unsafe path: {0}")]
    UnsafePath(&'static str),
    /// The peer broke the transfer protocol.
    #[error("protocol violation: {0}")]
    Protocol(&'static str),
    /// The receiving device declined the transfer.
    #[error("the other device declined: {0}")]
    Rejected(String),
    /// A file is too large or there are too many files.
    #[error("too large: {0}")]
    TooLarge(&'static str),
    /// The transfer could not finish after all retries.
    #[error("transfer interrupted: {0}")]
    Interrupted(String),
}

impl Error {
    pub(crate) fn io(action: &'static str, path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            action,
            path: path.to_path_buf(),
            source,
        }
    }
}

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
