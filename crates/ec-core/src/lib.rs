//! Device identity, pairing and the trust store of paired devices.
//!
//! Every device owns a long-term Ed25519 key and a self-signed certificate made from it.
//! The BLAKE3 hash of that certificate is the device's [`DeviceId`]. Peers pin this id
//! after pairing, so a connection is only accepted from the exact certificate that was
//! paired. See `docs/THREAT_MODEL.md` for the reasoning.

mod error;
mod fsutil;
mod id;
mod identity;
mod invite;
pub mod pairing;
mod text;
mod trust;

pub use error::{Error, Result};
pub use id::DeviceId;
pub use identity::{CERT_SERVER_NAME, DeviceIdentity};
pub use invite::{MAX_INVITE_ADDRS, PairingInvite};
pub use pairing::PairingSecret;
pub use text::sanitize_device_name;
pub use trust::{TrustStore, TrustedDevice};
