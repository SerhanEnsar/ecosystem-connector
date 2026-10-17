//! The pairing protocol.
//!
//! ```text
//! initiator (scanned the QR)              responder (shows the QR)
//!   TLS: pins responder's DeviceId  ───►  TLS: accepts any client certificate
//!   Request { name, proof }         ───►  checks proof, otherwise Rejected and stop
//!                                   ◄───  Accepted { name }
//!   both show the verification code and ask the user
//!   Decision { accept }             ◄──►  Decision { accept }
//! ```
//!
//! The invite secret is single use: the first connection that sends a request ends the
//! pairing window, whether the proof is valid or not.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use ec_core::pairing::{CHANNEL_BINDING_LABEL, proof, verification_code, verify_proof};
use ec_core::{
    CERT_SERVER_NAME, DeviceId, DeviceIdentity, PairingInvite, PairingSecret, sanitize_device_name,
};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::frame::{read_msg, write_msg};
use crate::tls::{self, ClientPolicy};
use crate::{ALPN_PAIRING, Error, Result};

/// How long the pairing listener waits for a device to connect.
pub const PAIRING_WINDOW: Duration = Duration::from_secs(120);
/// How long each side waits for its user and for the other side's decision.
pub const DECISION_TIMEOUT: Duration = Duration::from_secs(90);
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Serialize, Deserialize)]
enum PairMsg {
    /// `port` is the sender's regular node port, so the other side can reach it later
    /// without mDNS.
    Request {
        name: String,
        proof: [u8; 32],
        port: u16,
    },
    Accepted {
        name: String,
        port: u16,
    },
    Rejected,
    Decision {
        accept: bool,
    },
}

/// What to show the user before they confirm pairing.
#[derive(Clone, Debug)]
pub struct PairingPrompt {
    /// The other device's id.
    pub peer: DeviceId,
    /// The other device's name (sanitized; chosen by that device, so not proof of anything).
    pub peer_name: String,
    /// The code that must match the one on the other device, for example `"042 917"`.
    pub code: String,
}

/// A device both users agreed to pair with. Add it to the trust store.
#[derive(Clone, Debug)]
pub struct PairedDevice {
    /// Its id.
    pub id: DeviceId,
    /// Its name.
    pub name: String,
    /// Where its node listens, as seen during pairing (its IP and announced port).
    pub addr: Option<SocketAddr>,
}

fn node_addr(conn: &quinn::Connection, port: u16) -> Option<SocketAddr> {
    (port != 0).then(|| SocketAddr::new(conn.remote_address().ip(), port))
}

/// Listens for one pairing attempt.
pub struct PairingListener {
    identity: Arc<DeviceIdentity>,
    endpoint: quinn::Endpoint,
    secret: PairingSecret,
    node_port: u16,
}

impl PairingListener {
    /// Opens the listener on `addr` with a fresh secret. `node_port` is this device's
    /// regular node port, told to the other device during pairing.
    pub fn bind(identity: Arc<DeviceIdentity>, addr: SocketAddr, node_port: u16) -> Result<Self> {
        let server = tls::server_config(&identity, ClientPolicy::AnyForPairing, ALPN_PAIRING)?;
        let endpoint = quinn::Endpoint::server(server, addr)?;
        Ok(Self {
            identity,
            endpoint,
            secret: PairingSecret::generate(),
            node_port,
        })
    }

    /// The port the listener is bound to.
    pub fn port(&self) -> Result<u16> {
        Ok(self.endpoint.local_addr()?.port())
    }

    /// The invite to show as a QR code, listing `ips` with this listener's port.
    pub fn invite(&self, ips: &[std::net::IpAddr]) -> Result<PairingInvite> {
        let port = self.port()?;
        Ok(PairingInvite {
            responder: self.identity.id(),
            secret: self.secret.clone(),
            addrs: ips
                .iter()
                .take(ec_core::MAX_INVITE_ADDRS)
                .map(|ip| SocketAddr::new(*ip, port))
                .collect(),
        })
    }

    /// Waits for a device to use the invite, asks the user with `confirm`, and returns
    /// the paired device if both users accepted.
    pub async fn accept<F, Fut>(self, confirm: F) -> Result<PairedDevice>
    where
        F: FnOnce(PairingPrompt) -> Fut,
        Fut: Future<Output = bool>,
    {
        let result = timeout(PAIRING_WINDOW, self.wait_for_request())
            .await
            .map_err(|_| Error::PairingFailed("no device connected in time"))?;
        let ValidRequest {
            conn,
            mut send,
            mut recv,
            peer,
            name,
            binding,
            port,
        } = match result? {
            Request::Valid(r) => r,
            Request::BadProof(conn, mut send) => {
                let _ = write_msg(&mut send, &PairMsg::Rejected).await;
                finish(&mut send).await;
                conn.close(0u32.into(), b"rejected");
                self.endpoint.wait_idle().await;
                return Err(Error::PairingFailed(
                    "the other device did not have this invite's secret",
                ));
            }
        };

        write_msg(
            &mut send,
            &PairMsg::Accepted {
                name: self.identity.name().to_owned(),
                port: self.node_port,
            },
        )
        .await?;
        let me = self.identity.id();
        let prompt = PairingPrompt {
            peer,
            peer_name: name.clone(),
            code: verification_code(&self.secret, &peer, &me, &binding),
        };
        let outcome = decide(&mut send, &mut recv, confirm(prompt)).await;
        let _ = timeout(Duration::from_secs(2), conn.closed()).await;
        self.endpoint.close(0u32.into(), b"done");
        let addr = node_addr(&conn, port);
        outcome.map(|()| PairedDevice {
            id: peer,
            name,
            addr,
        })
    }

    async fn wait_for_request(&self) -> Result<Request> {
        let me = self.identity.id();
        loop {
            let incoming = self
                .endpoint
                .accept()
                .await
                .ok_or(Error::PairingFailed("listener closed"))?;
            // Failed handshakes and silent connections do not use up the invite.
            let Ok(conn) = incoming.await else { continue };
            let Some(peer) = tls::peer_id(&conn) else {
                continue;
            };
            let Ok(binding) = channel_binding(&conn) else {
                continue;
            };
            let Ok(Ok((send, mut recv))) = timeout(STEP_TIMEOUT, conn.accept_bi()).await else {
                continue;
            };
            let Ok(Ok(msg)) = timeout(STEP_TIMEOUT, read_msg::<PairMsg>(&mut recv)).await else {
                continue;
            };
            let PairMsg::Request { name, proof, port } = msg else {
                continue;
            };
            return Ok(
                if verify_proof(&self.secret, &peer, &me, &binding, &proof) {
                    Request::Valid(ValidRequest {
                        conn,
                        send,
                        recv,
                        peer,
                        name: sanitize_device_name(&name),
                        binding,
                        port,
                    })
                } else {
                    Request::BadProof(conn, send)
                },
            );
        }
    }
}

struct ValidRequest {
    conn: quinn::Connection,
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    peer: DeviceId,
    name: String,
    binding: [u8; 32],
    port: u16,
}

enum Request {
    Valid(ValidRequest),
    BadProof(quinn::Connection, quinn::SendStream),
}

/// Uses `invite` to pair with the device that created it. `node_port` is this
/// device's regular node port, told to the other device.
pub async fn pair_with<F, Fut>(
    identity: Arc<DeviceIdentity>,
    invite: &PairingInvite,
    node_port: u16,
    confirm: F,
) -> Result<PairedDevice>
where
    F: FnOnce(PairingPrompt) -> Fut,
    Fut: Future<Output = bool>,
{
    let endpoint = quinn::Endpoint::client(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)))?;
    let config = tls::client_config(&identity, invite.responder, ALPN_PAIRING)?;

    let mut conn = None;
    for addr in invite.addrs.iter().filter(|a| a.is_ipv4()) {
        let connecting = endpoint.connect_with(config.clone(), *addr, CERT_SERVER_NAME)?;
        if let Ok(Ok(c)) = timeout(Duration::from_secs(3), connecting).await {
            conn = Some(c);
            break;
        }
    }
    let conn = conn.ok_or(Error::Unreachable)?;

    let me = identity.id();
    let binding = channel_binding(&conn)?;
    let (mut send, mut recv) = conn.open_bi().await?;
    let request = PairMsg::Request {
        name: identity.name().to_owned(),
        proof: proof(&invite.secret, &me, &invite.responder, &binding),
        port: node_port,
    };
    write_msg(&mut send, &request).await?;

    let (name, port) = match timeout(STEP_TIMEOUT, read_msg(&mut recv)).await?? {
        PairMsg::Accepted { name, port } => (sanitize_device_name(&name), port),
        PairMsg::Rejected => {
            return Err(Error::PairingFailed("the other device rejected the invite"));
        }
        _ => return Err(Error::Protocol("unexpected pairing message")),
    };
    let prompt = PairingPrompt {
        peer: invite.responder,
        peer_name: name.clone(),
        code: verification_code(&invite.secret, &me, &invite.responder, &binding),
    };
    let outcome = decide(&mut send, &mut recv, confirm(prompt)).await;
    conn.close(0u32.into(), b"done");
    let _ = timeout(Duration::from_secs(2), endpoint.wait_idle()).await;
    let addr = node_addr(&conn, port);
    outcome.map(|()| PairedDevice {
        id: invite.responder,
        name,
        addr,
    })
}

/// Sends our user's decision, reads the other side's, and succeeds only if both accept.
async fn decide(
    send: &mut quinn::SendStream,
    recv: &mut quinn::RecvStream,
    confirm: impl Future<Output = bool>,
) -> Result<()> {
    let accept = timeout(DECISION_TIMEOUT, confirm).await.unwrap_or(false);
    write_msg(send, &PairMsg::Decision { accept }).await?;
    let peer_accept = match timeout(DECISION_TIMEOUT, read_msg(recv)).await?? {
        PairMsg::Decision { accept } => accept,
        _ => return Err(Error::Protocol("expected decision")),
    };
    finish(send).await;
    match (accept, peer_accept) {
        (true, true) => Ok(()),
        (false, _) => Err(Error::PairingFailed("declined on this device")),
        (true, false) => Err(Error::PairingFailed("declined on the other device")),
    }
}

async fn finish(send: &mut quinn::SendStream) {
    let _ = send.finish();
    let _ = timeout(Duration::from_secs(2), send.stopped()).await;
}

/// Keying material unique to this TLS session. Including it in the proof and code means
/// they cannot be replayed on, or relayed to, another connection.
fn channel_binding(conn: &quinn::Connection) -> Result<[u8; 32]> {
    let mut out = [0u8; 32];
    conn.export_keying_material(&mut out, CHANNEL_BINDING_LABEL, b"")
        .map_err(|_| Error::Protocol("keying material export failed"))?;
    Ok(out)
}
