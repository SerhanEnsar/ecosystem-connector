//! Routes the streams of a connection to services.
//!
//! The first byte of every stream names the service it belongs to (its *kind*). Higher
//! layers, such as file transfer, register a [`Service`] for their kinds, and
//! [`serve_connection`] hands each incoming stream to the right one. Streams of an
//! unknown kind are refused.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use quinn::{RecvStream, SendStream};
use tokio::time::timeout;

use crate::node::{PeerConnection, SessionService};
use crate::{Error, Result};

/// Stream kind of the built-in session (hello and ping).
pub const SESSION_KIND: u8 = 0;

const KIND_TIMEOUT: Duration = Duration::from_secs(10);

/// The future a service returns for one stream. Services handle their own errors.
pub type ServiceFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// Something that answers streams of one or more kinds.
pub trait Service: Send + Sync + 'static {
    /// Handles a bidirectional stream. The kind byte has already been read.
    fn handle_bi(
        self: Arc<Self>,
        conn: PeerConnection,
        send: SendStream,
        recv: RecvStream,
    ) -> ServiceFuture {
        let _ = (conn, send, recv);
        Box::pin(async {})
    }

    /// Handles a unidirectional stream. The kind byte has already been read.
    fn handle_uni(self: Arc<Self>, conn: PeerConnection, recv: RecvStream) -> ServiceFuture {
        let _ = (conn, recv);
        Box::pin(async {})
    }
}

/// Maps stream kinds to services.
#[derive(Default, Clone)]
pub struct Router {
    services: HashMap<u8, Arc<dyn Service>>,
}

impl Router {
    /// A router that only answers sessions (hello and ping) as `my_name`.
    ///
    /// `on_hello` is called with each peer's announced (sanitized) name.
    pub fn with_session(
        my_name: impl Into<String>,
        on_hello: impl Fn(ec_core::DeviceId, &str) + Send + Sync + 'static,
    ) -> Self {
        let mut router = Self::default();
        let session = SessionService::new(my_name.into(), on_hello);
        router.services.insert(SESSION_KIND, Arc::new(session));
        router
    }

    /// Registers `service` for `kind`. Fails if the kind is already taken.
    pub fn add(&mut self, kind: u8, service: Arc<dyn Service>) -> Result<()> {
        if self.services.contains_key(&kind) {
            return Err(Error::Protocol("stream kind registered twice"));
        }
        self.services.insert(kind, service);
        Ok(())
    }

    fn get(&self, kind: u8) -> Option<Arc<dyn Service>> {
        self.services.get(&kind).cloned()
    }
}

/// Answers the streams of `conn` with `router` until the connection closes.
pub async fn serve_connection(conn: PeerConnection, router: Arc<Router>) -> Result<()> {
    let quic = conn.quic().clone();
    loop {
        tokio::select! {
            bi = quic.accept_bi() => {
                let (send, mut recv) = match bi {
                    Ok(streams) => streams,
                    Err(e) => return closed_ok(e),
                };
                let (conn, router) = (conn.clone(), router.clone());
                tokio::spawn(async move {
                    if let Some(service) = route(&router, &mut recv).await {
                        service.handle_bi(conn, send, recv).await;
                    }
                });
            }
            uni = quic.accept_uni() => {
                let mut recv = match uni {
                    Ok(stream) => stream,
                    Err(e) => return closed_ok(e),
                };
                let (conn, router) = (conn.clone(), router.clone());
                tokio::spawn(async move {
                    if let Some(service) = route(&router, &mut recv).await {
                        service.handle_uni(conn, recv).await;
                    }
                });
            }
        }
    }
}

async fn route(router: &Router, recv: &mut RecvStream) -> Option<Arc<dyn Service>> {
    let mut kind = [0u8; 1];
    let read = timeout(KIND_TIMEOUT, recv.read_exact(&mut kind)).await;
    let service = match read {
        Ok(Ok(())) => router.get(kind[0]),
        _ => None,
    };
    if service.is_none() {
        let _ = recv.stop(0u32.into());
    }
    service
}

fn closed_ok(e: quinn::ConnectionError) -> Result<()> {
    match e {
        quinn::ConnectionError::ApplicationClosed(_)
        | quinn::ConnectionError::LocallyClosed
        | quinn::ConnectionError::TimedOut => Ok(()),
        e => Err(e.into()),
    }
}
