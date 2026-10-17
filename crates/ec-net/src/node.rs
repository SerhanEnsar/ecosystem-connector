use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use ec_core::{CERT_SERVER_NAME, DeviceId, DeviceIdentity, sanitize_device_name};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use crate::frame::{read_msg, write_msg};
use crate::tls::{self, ClientPolicy, TrustedSet};
use crate::{ALPN_SESSION, Error, Result};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// Messages on a session stream.
#[derive(Debug, Serialize, Deserialize)]
enum SessionMsg {
    /// First message in each direction.
    Hello { name: String },
    /// Liveness and latency check.
    Ping { nonce: u64 },
    /// Answer to [`SessionMsg::Ping`].
    Pong { nonce: u64 },
}

/// The regular QUIC endpoint of a device.
///
/// Incoming handshakes from devices that are not in the trusted set are refused by
/// TLS itself, before any application data is exchanged.
pub struct Node {
    identity: Arc<DeviceIdentity>,
    trusted: TrustedSet,
    endpoint: quinn::Endpoint,
}

impl Node {
    /// Binds a node to `addr` (use port 0 for any free port).
    pub fn bind(
        identity: Arc<DeviceIdentity>,
        trusted: impl IntoIterator<Item = DeviceId>,
        addr: SocketAddr,
    ) -> Result<Self> {
        let trusted: TrustedSet = Arc::new(RwLock::new(trusted.into_iter().collect()));
        let server = tls::server_config(
            &identity,
            ClientPolicy::Trusted(trusted.clone()),
            ALPN_SESSION,
        )?;
        let endpoint = quinn::Endpoint::server(server, addr)?;
        Ok(Self {
            identity,
            trusted,
            endpoint,
        })
    }

    /// The address the node listens on.
    pub fn local_addr(&self) -> Result<SocketAddr> {
        Ok(self.endpoint.local_addr()?)
    }

    /// Allows connections with `id` from now on.
    pub fn trust(&self, id: DeviceId) {
        if let Ok(mut set) = self.trusted.write() {
            set.insert(id);
        }
    }

    /// Refuses new connections with `id` from now on.
    pub fn untrust(&self, id: &DeviceId) {
        if let Ok(mut set) = self.trusted.write() {
            set.remove(id);
        }
    }

    fn is_trusted(&self, id: &DeviceId) -> bool {
        self.trusted.read().map(|s| s.contains(id)).unwrap_or(false)
    }

    /// Connects to the paired device `peer`, trying `addrs` in order.
    ///
    /// The handshake only succeeds if the server presents the certificate whose hash is
    /// `peer`, so a different device answering on that address is rejected.
    pub async fn connect(&self, peer: DeviceId, addrs: &[SocketAddr]) -> Result<PeerConnection> {
        if !self.is_trusted(&peer) {
            return Err(Error::NotPaired(peer));
        }
        let config = tls::client_config(&self.identity, peer, ALPN_SESSION)?;
        let mut last_error = Error::Unreachable;
        for addr in addrs {
            let connecting = self
                .endpoint
                .connect_with(config.clone(), *addr, CERT_SERVER_NAME)?;
            match timeout(CONNECT_TIMEOUT, connecting).await {
                Ok(Ok(conn)) => return Ok(PeerConnection { conn, peer }),
                Ok(Err(e)) => last_error = e.into(),
                Err(_) => last_error = Error::Timeout,
            }
        }
        Err(last_error)
    }

    /// Waits for the next incoming connection. Returns `None` once the node is closed.
    ///
    /// A handshake that fails (for example from an unpaired device) is returned as an
    /// error; callers should log it and keep accepting.
    pub async fn accept(&self) -> Option<Result<PeerConnection>> {
        let incoming = self.endpoint.accept().await?;
        Some(
            async {
                let conn = incoming.await?;
                let peer =
                    tls::peer_id(&conn).ok_or(Error::Protocol("missing peer certificate"))?;
                Ok(PeerConnection { conn, peer })
            }
            .await,
        )
    }

    /// Closes all connections and waits until they are shut down.
    pub async fn shutdown(&self) {
        self.endpoint.close(0u32.into(), b"shutdown");
        let _ = timeout(Duration::from_secs(2), self.endpoint.wait_idle()).await;
    }
}

/// An authenticated connection with a paired device.
#[derive(Clone)]
pub struct PeerConnection {
    conn: quinn::Connection,
    peer: DeviceId,
}

impl PeerConnection {
    /// The peer's device id, proven by the TLS handshake.
    pub fn peer(&self) -> DeviceId {
        self.peer
    }

    /// The peer's current network address.
    pub fn remote_addr(&self) -> SocketAddr {
        self.conn.remote_address()
    }

    /// Opens a session stream and exchanges names.
    pub async fn open_session(&self, my_name: &str) -> Result<Session> {
        let (mut send, mut recv) = self.conn.open_bi().await?;
        write_msg(
            &mut send,
            &SessionMsg::Hello {
                name: my_name.to_owned(),
            },
        )
        .await?;
        let peer_name = match timeout(HELLO_TIMEOUT, read_msg(&mut recv)).await?? {
            SessionMsg::Hello { name } => sanitize_device_name(&name),
            _ => return Err(Error::Protocol("expected hello")),
        };
        Ok(Session {
            send,
            recv,
            peer_name,
            next_nonce: 0,
        })
    }

    /// Closes the connection.
    pub fn close(&self) {
        self.conn.close(0u32.into(), b"bye");
    }
}

/// A session stream opened by [`PeerConnection::open_session`].
pub struct Session {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    peer_name: String,
    /// Matches pongs to pings; not security relevant.
    next_nonce: u64,
}

impl Session {
    /// The name the peer announced (sanitized).
    pub fn peer_name(&self) -> &str {
        &self.peer_name
    }

    /// Sends a ping and returns the round-trip time.
    pub async fn ping(&mut self) -> Result<Duration> {
        let nonce = self.next_nonce;
        self.next_nonce = self.next_nonce.wrapping_add(1);
        let start = Instant::now();
        write_msg(&mut self.send, &SessionMsg::Ping { nonce }).await?;
        match timeout(HELLO_TIMEOUT, read_msg(&mut self.recv)).await?? {
            SessionMsg::Pong { nonce: n } if n == nonce => Ok(start.elapsed()),
            _ => Err(Error::Protocol("unexpected answer to ping")),
        }
    }

    /// Ends the session cleanly.
    pub async fn finish(mut self) {
        let _ = self.send.finish();
        let _ = timeout(Duration::from_secs(2), self.send.stopped()).await;
    }
}

/// Answers session streams on `conn` until the peer closes it.
///
/// `my_name` is sent in the hello. `on_hello` is called with each peer's announced name.
pub async fn serve_connection(
    conn: PeerConnection,
    my_name: String,
    on_hello: impl Fn(DeviceId, &str) + Send + Sync + 'static,
) -> Result<()> {
    let on_hello = Arc::new(on_hello);
    loop {
        let (send, recv) = match conn.conn.accept_bi().await {
            Ok(streams) => streams,
            Err(
                quinn::ConnectionError::ApplicationClosed(_)
                | quinn::ConnectionError::LocallyClosed
                | quinn::ConnectionError::TimedOut,
            ) => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let name = my_name.clone();
        let peer = conn.peer;
        let on_hello = on_hello.clone();
        tokio::spawn(async move {
            if let Err(e) = serve_session(send, recv, &name, peer, &*on_hello).await {
                tracing::debug!(peer = %peer.short(), "session ended: {e}");
            }
        });
    }
}

async fn serve_session(
    mut send: quinn::SendStream,
    mut recv: quinn::RecvStream,
    my_name: &str,
    peer: DeviceId,
    on_hello: &(dyn Fn(DeviceId, &str) + Send + Sync),
) -> Result<()> {
    match timeout(HELLO_TIMEOUT, read_msg(&mut recv)).await?? {
        SessionMsg::Hello { name } => on_hello(peer, &sanitize_device_name(&name)),
        _ => return Err(Error::Protocol("expected hello")),
    }
    write_msg(
        &mut send,
        &SessionMsg::Hello {
            name: my_name.to_owned(),
        },
    )
    .await?;
    loop {
        match read_msg(&mut recv).await {
            Ok(SessionMsg::Ping { nonce }) => {
                write_msg(&mut send, &SessionMsg::Pong { nonce }).await?
            }
            Ok(_) => return Err(Error::Protocol("unexpected message")),
            Err(Error::Read(quinn::ReadExactError::FinishedEarly(0))) => return Ok(()),
            Err(e) => return Err(e),
        }
    }
}
