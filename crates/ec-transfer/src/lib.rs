//! Chunked, parallel and resumable file transfer with BLAKE3 integrity checks.
//!
//! - [`prepare`] lists and hashes the files to send.
//! - [`send_prepared`] / [`send_paths`] send them over one or more paths.
//! - [`Receiver`] is the service that saves incoming files; register it with an
//!   [`ec_net::Router`] for [`CONTROL_KIND`] and [`CHUNK_KIND`].
//!
//! The wire format is described in `docs/PROTOCOL.md`.

mod error;
mod fileio;
pub mod manifest;
pub mod path;
pub mod proto;
mod receiver;
mod sender;

pub use error::{Error, Result};
pub use manifest::{FileEntry, SourceFile, prepare};
pub use path::RelPath;
pub use proto::{CHUNK_KIND, CONTROL_KIND};
pub use receiver::{AcceptPolicy, Offer, PARTIAL_DIR, ReceiveEvent, ReceiveEvents, Receiver};
pub use sender::{
    AddrSource, SendEvent, SendEvents, SendOptions, SendReport, send_paths, send_prepared,
};
