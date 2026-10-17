//! Local network discovery (mDNS) and the encrypted QUIC transport between paired devices.
//!
//! Two kinds of QUIC endpoints exist:
//!
//! - [`Node`] is the regular endpoint. It only completes handshakes with devices in the
//!   trust store, and it only connects to a device whose certificate hash matches the
//!   expected [`DeviceId`](ec_core::DeviceId).
//! - [`pairing::PairingListener`] is opened only while the user is pairing. It accepts
//!   any client certificate, but the client must prove it knows the one-time secret
//!   from the invite before anything else happens.
//!
//! Every connection uses TLS 1.3 with Ed25519 certificates and mutual authentication.

mod discovery;
mod error;
mod frame;
mod node;
pub mod pairing;
mod router;
mod tls;

pub use discovery::{Advertiser, DiscoveredDevice, Presence, SERVICE_TYPE, browse, local_addrs};
pub use error::{Error, Result};
pub use frame::{MAX_FRAME_LEN, decode, read_msg, write_msg};
pub use node::{Node, PeerConnection, Session};
pub use router::{Router, SESSION_KIND, Service, ServiceFuture, serve_connection};

/// QUIC stream types used by services.
pub use quinn::{RecvStream, SendStream};

/// ALPN protocol id of regular connections.
pub(crate) const ALPN_SESSION: &[u8] = b"ec/1";
/// ALPN protocol id of pairing connections.
pub(crate) const ALPN_PAIRING: &[u8] = b"ec-pair/1";
