use std::path::PathBuf;

/// Errors returned by this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reading or writing a file failed.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was being done, for example "read".
        action: &'static str,
        /// The file involved.
        path: PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A stored file exists but its contents are not valid.
    #[error("invalid data in {path}: {reason}")]
    Corrupt {
        /// The file involved.
        path: PathBuf,
        /// Why it was rejected.
        reason: String,
    },
    /// Generating the key or certificate failed.
    #[error("certificate generation failed: {0}")]
    Certificate(#[from] rcgen::Error),
    /// A pairing invite could not be parsed.
    #[error("invalid pairing invite: {0}")]
    InvalidInvite(&'static str),
    /// A device id could not be parsed.
    #[error("invalid device id")]
    InvalidDeviceId,
}

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
