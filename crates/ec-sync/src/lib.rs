//! Linked files: one logical file kept current on every paired device.
//!
//! Each linked file has an id shared by all its devices and a version vector: every
//! device numbers its own changes, so devices can tell whether a version follows
//! another or was made independently (a conflict) without comparing clocks. The
//! [`journal`] keeps these records and a short change log on disk. Design:
//! `docs/superpowers/specs/2026-10-19-linked-files-design.md`.

use std::path::PathBuf;

pub mod journal;
pub mod proto;
mod version;

pub use journal::{
    Content, FileIdentity, Journal, LinkId, LinkedFile, LogEntry, LogKind, Seen, State, new_link_id,
};
pub use proto::{ContentHeader, SYNC_KIND, SyncMessage};
pub use version::{Order, VersionVector};

/// Errors of this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Storage error from `ec-core`.
    #[error(transparent)]
    Core(#[from] ec_core::Error),
    /// A file could not be read or written.
    #[error("cannot {action} {path}: {source}")]
    Io {
        /// What was being done.
        action: &'static str,
        /// The file involved.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// A record could not be encoded.
    #[error("cannot encode {0}")]
    Encode(&'static str),
    /// Network error.
    #[error(transparent)]
    Net(#[from] ec_net::Error),
    /// A message from a peer was refused.
    #[error("refused: {0}")]
    Refused(&'static str),
}

/// Result type of this crate.
pub type Result<T> = std::result::Result<T, Error>;
