//! Everything a running device needs, in one place: its identity and trust store, the
//! QUIC node, presence on the network, pairing, and sending and receiving files.
//!
//! The desktop and mobile apps use this through `ec-ffi`; it has no FFI or UI code of
//! its own, so it can be tested directly.

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ec_core::{DeviceId, DeviceIdentity, PairingInvite, TrustStore, TrustedDevice};
use ec_net::pairing::{PairedDevice, PairingListener, PairingPrompt, pair_with};
use ec_net::{DiscoveredDevice, Node, Presence, Router, local_addrs, serve_connection};
use ec_transfer::{
    CHUNK_KIND, CONTROL_KIND, ReceiveEvent, Receiver, SendEvents, SendOptions, SendReport,
    send_paths,
};
use tokio::sync::broadcast;

/// Errors returned by the runtime.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Identity or trust store error.
    #[error(transparent)]
    Core(#[from] ec_core::Error),
    /// Network error.
    #[error(transparent)]
    Net(#[from] ec_net::Error),
    /// Transfer error.
    #[error(transparent)]
    Transfer(#[from] ec_transfer::Error),
    /// The device is not paired.
    #[error("this device is not paired")]
    NotPaired,
    /// The device is paired but not on the network right now.
    #[error("the device is not on the network")]
    Offline,
}

/// Result type of the runtime.
pub type Result<T> = std::result::Result<T, Error>;

/// How to start a runtime.
#[derive(Clone, Debug)]
pub struct Config {
    /// Folder for the identity and the trust store.
    pub home: PathBuf,
    /// Device name used when the identity is created for the first time.
    pub default_name: String,
    /// Folder for received files.
    pub receive_dir: PathBuf,
    /// UDP port; 0 picks a free one.
    pub port: u16,
}

/// Something the user interface may want to show.
#[derive(Clone, Debug)]
pub enum Event {
    /// Paired devices or their online state changed; read [`Runtime::devices`] again.
    DevicesChanged,
    /// A new device was paired.
    Paired {
        /// Its id.
        id: DeviceId,
        /// Its name.
        name: String,
    },
    /// Something happened while receiving files.
    Receive(ReceiveEvent),
}

/// A paired device and whether it is reachable now.
#[derive(Clone, Debug)]
pub struct DeviceStatus {
    /// Its id.
    pub id: DeviceId,
    /// Its name.
    pub name: String,
    /// Whether it is announcing itself on the network.
    pub online: bool,
    /// Its current addresses, empty when offline.
    pub addrs: Vec<SocketAddr>,
}

/// A running device.
pub struct Runtime {
    identity: Arc<DeviceIdentity>,
    trust: Mutex<TrustStore>,
    node: Arc<Node>,
    presence: Arc<Presence>,
    events: broadcast::Sender<Event>,
    accept_task: tokio::task::JoinHandle<()>,
}

impl Runtime {
    /// Loads (or creates) the identity, starts listening, announcing and watching the
    /// network, and accepts files from paired devices. Must run inside a tokio runtime.
    pub async fn start(config: Config) -> Result<Arc<Self>> {
        let identity = Arc::new(DeviceIdentity::load_or_create(
            &config.home,
            &config.default_name,
        )?);
        let trust = TrustStore::load(&config.home)?;
        let node = Arc::new(Node::bind(
            identity.clone(),
            trust.devices().map(|d| d.id),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port)),
        )?);
        let (events, _) = broadcast::channel(256);

        let changes = events.clone();
        let port = node.local_addr()?.port();
        let presence = Arc::new(Presence::start(&identity, port, move || {
            let _ = changes.send(Event::DevicesChanged);
        })?);

        let receive_events = events.clone();
        let receiver = Arc::new(Receiver::new(
            config.receive_dir.clone(),
            // Only paired devices can connect at all, so their transfers are accepted.
            Arc::new(|_| true),
            Arc::new(move |event| {
                let _ = receive_events.send(Event::Receive(event));
            }),
        ));
        let mut router = Router::with_session(identity.name(), |_, _| {});
        router.add(CONTROL_KIND, receiver.clone())?;
        router.add(CHUNK_KIND, receiver)?;
        let router = Arc::new(router);

        let accepting = node.clone();
        let accept_task = tokio::spawn(async move {
            while let Some(conn) = accepting.accept().await {
                match conn {
                    Ok(conn) => {
                        tokio::spawn(serve_connection(conn, router.clone()));
                    }
                    Err(e) => tracing::info!("refused a connection: {e}"),
                }
            }
        });

        Ok(Arc::new(Self {
            identity,
            trust: Mutex::new(trust),
            node,
            presence,
            events,
            accept_task,
        }))
    }

    /// This device's id.
    pub fn id(&self) -> DeviceId {
        self.identity.id()
    }

    /// This device's name.
    pub fn name(&self) -> &str {
        self.identity.name()
    }

    /// Subscribes to [`Event`]s.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    fn trust(&self) -> std::sync::MutexGuard<'_, TrustStore> {
        self.trust.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Paired devices, with their online state.
    pub fn devices(&self) -> Vec<DeviceStatus> {
        let paired: Vec<TrustedDevice> = self.trust().devices().cloned().collect();
        paired
            .into_iter()
            .map(|device| {
                let seen = self.presence.get(&device.id);
                DeviceStatus {
                    id: device.id,
                    name: seen.as_ref().map_or(device.name, |s| s.name.clone()),
                    online: seen.is_some(),
                    addrs: seen.map(|s| s.addrs).unwrap_or_default(),
                }
            })
            .collect()
    }

    /// Devices on the network that are not paired with this one.
    pub fn nearby_unpaired(&self) -> Vec<DiscoveredDevice> {
        let trust = self.trust();
        self.presence
            .devices()
            .into_iter()
            .filter(|d| !trust.contains(&d.id))
            .collect()
    }

    /// Shows an invite (through `on_invite`, as a QR code or link) and waits for a
    /// device to use it. `confirm` shows the verification code to the user.
    pub async fn host_pairing<F, Fut>(
        &self,
        on_invite: impl FnOnce(String),
        confirm: F,
    ) -> Result<TrustedDevice>
    where
        F: FnOnce(PairingPrompt) -> Fut,
        Fut: Future<Output = bool>,
    {
        let listener = PairingListener::bind(
            self.identity.clone(),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0)),
        )?;
        on_invite(listener.invite(&local_addrs())?.to_uri());
        let paired = listener.accept(confirm).await?;
        self.remember(paired)
    }

    /// Pairs with the device that shows `invite`.
    pub async fn join_pairing<F, Fut>(&self, invite: &str, confirm: F) -> Result<TrustedDevice>
    where
        F: FnOnce(PairingPrompt) -> Fut,
        Fut: Future<Output = bool>,
    {
        let invite = PairingInvite::parse(invite)?;
        let paired = pair_with(self.identity.clone(), &invite, confirm).await?;
        self.remember(paired)
    }

    fn remember(&self, paired: PairedDevice) -> Result<TrustedDevice> {
        let device = self.trust().add(paired.id, &paired.name)?;
        self.node.trust(device.id);
        let _ = self.events.send(Event::Paired {
            id: device.id,
            name: device.name.clone(),
        });
        let _ = self.events.send(Event::DevicesChanged);
        Ok(device)
    }

    /// Removes a paired device. Its connections are refused from now on.
    pub fn forget(&self, id: &DeviceId) -> Result<bool> {
        let removed = self.trust().remove(id)?;
        self.node.untrust(id);
        let _ = self.events.send(Event::DevicesChanged);
        Ok(removed)
    }

    /// Sends files and folders to a paired, online device.
    pub async fn send(
        &self,
        id: DeviceId,
        paths: Vec<PathBuf>,
        events: SendEvents,
    ) -> Result<SendReport> {
        if !self.trust().contains(&id) {
            return Err(Error::NotPaired);
        }
        let addrs = self
            .presence
            .get(&id)
            .map(|d| d.addrs)
            .ok_or(Error::Offline)?;
        let presence = self.presence.clone();
        let rediscover: ec_transfer::AddrSource = Arc::new(move || {
            let addrs = presence.get(&id).map(|d| d.addrs).unwrap_or_default();
            Box::pin(async move { addrs })
        });
        let options = SendOptions {
            rediscover: Some(rediscover),
            ..SendOptions::default()
        };
        Ok(send_paths(self.node.clone(), id, addrs, paths, options, events).await?)
    }

    /// Stops listening and announcing.
    pub async fn shutdown(&self) {
        self.accept_task.abort();
        self.node.shutdown().await;
    }
}
