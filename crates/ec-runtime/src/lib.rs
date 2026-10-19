//! Everything a running device needs, in one place: its identity and trust store, the
//! QUIC node, presence on the network, pairing, and sending and receiving files.
//!
//! The desktop and mobile apps use this through `ec-ffi`; it has no FFI or UI code of
//! its own, so it can be tested directly.
//!
//! Paired devices are found in two ways. mDNS finds them on most home networks. Many
//! other networks (campus, office, some routers) do not deliver multicast, so the
//! runtime also remembers where each paired device was last reachable and checks those
//! addresses directly every few seconds. Either way, a connection only succeeds with the
//! device's pinned certificate.

use std::collections::HashMap;
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ec_core::{DeviceId, DeviceIdentity, PairingInvite, Settings, TrustStore, TrustedDevice};
use ec_fling::{FLING_KIND, FlingItem, FlingService};
use ec_net::pairing::{PairedDevice, PairingListener, PairingPrompt, pair_with};
use ec_net::{
    DiscoveredDevice, Node, PeerConnection, Presence, Router, local_addrs, serve_connection,
};
use ec_sync::watch::WatchConfig;
use ec_sync::{LinkId, LinkedFile, LogEntry, SYNC_KIND, SyncConfig, SyncEvent, SyncService};
use ec_transfer::{
    CHUNK_KIND, CONTROL_KIND, OpenHint, ReceiveEvent, Receiver, SendEvents, SendOptions,
    SendReport, send_paths,
};
use tokio::sync::broadcast;

pub use ec_net::DEFAULT_PORT;

/// How often paired devices that mDNS does not see are checked directly.
const PROBE_INTERVAL: Duration = Duration::from_secs(10);
/// How long a device counts as online after it was last reached directly.
const REACHED_FOR: Duration = Duration::from_secs(35);
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
/// How long a linked file must stay unchanged before it is checked.
const LINKED_QUIET: Duration = Duration::from_secs(2);
/// How often linked files are checked without a file system event. The apps lower it
/// where events are unreliable (Android).
const LINKED_CHECK_INTERVAL: Duration = Duration::from_secs(60);
/// How long a deleted linked file may take to be written again before it counts as
/// deleted (some apps save by deleting and rewriting).
const LINKED_DELETE_GRACE: Duration = Duration::from_secs(10);
/// The folder inside the receive folder that holds linked files from other devices.
pub const LINKED_DIR: &str = "Linked";

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
    /// Fling error.
    #[error(transparent)]
    Fling(#[from] ec_fling::Error),
    /// Linked files error.
    #[error(transparent)]
    Sync(#[from] ec_sync::Error),
    /// Linked files are turned off in this runtime.
    #[error("linked files are not available")]
    LinkedFilesOff,
    /// The device is not paired.
    #[error("this device is not paired")]
    NotPaired,
    /// No address is known for the device.
    #[error("the device is not on the network")]
    Offline,
    /// The chosen folder for received files cannot be used.
    #[error("cannot use {path} for received files: {source}")]
    ReceiveDir {
        /// The folder.
        path: PathBuf,
        /// Why.
        source: std::io::Error,
    },
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
    /// Default folder for received files. The user can choose another one with
    /// [`Runtime::set_receive_dir`]; that choice is saved in `home`.
    pub receive_dir: PathBuf,
    /// Preferred UDP port. If it is taken, a free one is used. 0 always picks a free one.
    pub port: u16,
    /// Whether to announce and look for devices with mDNS.
    pub mdns: bool,
    /// Linked files.
    pub linked: LinkedConfig,
}

/// Moves a file to the system Trash, or deletes it where there is none.
pub type TrashFn = ec_sync::service::TrashFn;

/// How linked files run.
#[derive(Clone)]
pub struct LinkedConfig {
    /// Whether this device offers linked files. Off only to test older peers.
    pub enabled: bool,
    /// Removes a file the user chose not to keep. `None` deletes it.
    pub trash: Option<TrashFn>,
    /// For tests: runs just before a received version replaces a linked file.
    #[doc(hidden)]
    pub before_commit: Option<ec_sync::service::CommitHook>,
}

impl Default for LinkedConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            trash: None,
            before_commit: None,
        }
    }
}

impl std::fmt::Debug for LinkedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkedConfig")
            .field("enabled", &self.enabled)
            .finish_non_exhaustive()
    }
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
    /// A paired device flung something to open here.
    Fling {
        /// The sending device.
        from: DeviceId,
        /// What it sent (already validated).
        item: FlingItem,
    },
    /// Something happened to a linked file.
    Sync(SyncEvent),
}

/// A paired device and whether it is reachable now.
#[derive(Clone, Debug)]
pub struct DeviceStatus {
    /// Its id.
    pub id: DeviceId,
    /// Its name.
    pub name: String,
    /// Whether it was seen on the network recently.
    pub online: bool,
    /// Its current addresses, empty when offline.
    pub addrs: Vec<SocketAddr>,
    /// Whether it supports linked files (known once it has connected).
    pub linked_files: bool,
}

/// State shared with the background tasks.
struct Shared {
    trust: Mutex<TrustStore>,
    /// Devices reached by a direct connection, with their address and when.
    reached: Mutex<HashMap<DeviceId, (SocketAddr, Instant)>>,
    /// One live connection per paired device, whichever side opened it. Both sides
    /// serve their services on it, so either can send even if only one of them can
    /// accept new connections.
    conns: Mutex<HashMap<DeviceId, PeerConnection>>,
    router: Arc<Router>,
    events: broadcast::Sender<Event>,
    sync: Option<Arc<SyncService>>,
}

impl Shared {
    fn live_conn(&self, id: &DeviceId) -> Option<PeerConnection> {
        let conns = self.conns.lock().unwrap_or_else(|e| e.into_inner());
        conns
            .get(id)
            .filter(|c| c.quic().close_reason().is_none())
            .cloned()
    }

    /// Keeps `conn` as the device's connection and answers its streams until it closes.
    fn adopt(self: &Arc<Self>, conn: PeerConnection) {
        let id = conn.peer();
        if !self.trust().contains(&id) {
            conn.close();
            return;
        }
        self.mark_reached(id, conn.remote_addr());
        self.conns
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, conn.clone());
        if let Some(sync) = &self.sync {
            sync.connected(id);
        }
        let shared = self.clone();
        tokio::spawn(async move {
            let stable_id = conn.quic().stable_id();
            if let Err(e) = serve_connection(conn, shared.router.clone()).await {
                tracing::debug!("connection with {} ended: {e}", id.short());
            }
            let mut conns = shared.conns.lock().unwrap_or_else(|e| e.into_inner());
            if conns
                .get(&id)
                .is_some_and(|c| c.quic().stable_id() == stable_id)
            {
                conns.remove(&id);
                drop(conns);
                let _ = shared.events.send(Event::DevicesChanged);
            }
        });
    }

    fn trust(&self) -> std::sync::MutexGuard<'_, TrustStore> {
        self.trust.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn reached_addr(&self, id: &DeviceId) -> Option<SocketAddr> {
        let reached = self.reached.lock().unwrap_or_else(|e| e.into_inner());
        reached
            .get(id)
            .filter(|(_, at)| at.elapsed() < REACHED_FOR)
            .map(|(addr, _)| *addr)
    }

    /// Records that `id` answered at `addr`, remembers the address, and tells the UI
    /// if the device just came online.
    fn mark_reached(&self, id: DeviceId, addr: SocketAddr) {
        if !self.trust().contains(&id) {
            return;
        }
        let was_online = self.reached_addr(&id).is_some();
        self.reached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, (addr, Instant::now()));
        if let Err(e) = self.trust().set_addrs(&id, &[addr]) {
            tracing::warn!("cannot remember the address of {}: {e}", id.short());
        }
        if !was_online {
            let _ = self.events.send(Event::DevicesChanged);
        }
    }
}

/// Creates `dir` if needed and checks that it is a folder.
fn prepare_dir(dir: &Path) -> Result<()> {
    let fail = |source| Error::ReceiveDir {
        path: dir.to_path_buf(),
        source,
    };
    std::fs::create_dir_all(dir).map_err(fail)?;
    if !dir.is_dir() {
        return Err(fail(std::io::Error::other("not a folder")));
    }
    Ok(())
}

/// A running device.
pub struct Runtime {
    identity: Arc<DeviceIdentity>,
    shared: Arc<Shared>,
    node: Arc<Node>,
    presence: Option<Arc<Presence>>,
    receiver: Arc<Receiver>,
    sync: Option<Arc<SyncService>>,
    home: PathBuf,
    default_receive_dir: PathBuf,
    settings: Mutex<Settings>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
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
        let settings = Settings::load(&config.home)?;
        // A chosen folder that has gone away (an unplugged drive, say) falls back to
        // the default rather than failing every transfer.
        let receive_dir = match &settings.receive_dir {
            Some(dir) if dir.is_dir() => dir.clone(),
            Some(dir) => {
                tracing::warn!("{} is missing, using the default folder", dir.display());
                config.receive_dir.clone()
            }
            None => config.receive_dir.clone(),
        };
        let node = Arc::new(bind_node(&identity, &trust, config.port)?);
        let (events, _) = broadcast::channel(256);

        let presence = if config.mdns {
            let changes = events.clone();
            let port = node.local_addr()?.port();
            Some(Arc::new(Presence::start(&identity, port, move || {
                let _ = changes.send(Event::DevicesChanged);
            })?))
        } else {
            None
        };

        let receive_events = events.clone();
        let receiver = Arc::new(Receiver::new(
            receive_dir,
            // Only paired devices can connect at all, so their transfers are accepted.
            Arc::new(|_| true),
            Arc::new(move |event| {
                let _ = receive_events.send(Event::Receive(event));
            }),
        ));
        // The service reaches other devices through `Shared`, which is made after the
        // router that serves it; a weak link avoids a cycle.
        let shared_slot: Arc<OnceLock<Weak<Shared>>> = Arc::new(OnceLock::new());
        let mut tasks = Vec::new();
        let sync = if config.linked.enabled {
            let (sync_events, mut sync_rx) = broadcast::channel(256);
            let linked_root = receiver.dir().join(LINKED_DIR);
            let cfg = SyncConfig {
                me: identity.id(),
                dir: config.home.join("linked"),
                linked_root: linked_root.clone(),
                trash: config
                    .linked
                    .trash
                    .clone()
                    .unwrap_or_else(|| Arc::new(|path: &Path| std::fs::remove_file(path))),
                now_ms: Arc::new(|| {
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |d| d.as_millis() as u64)
                }),
                watch: WatchConfig {
                    quiet: LINKED_QUIET,
                    stat_interval: LINKED_CHECK_INTERVAL,
                    roots: vec![linked_root],
                    delete_grace: LINKED_DELETE_GRACE,
                },
                before_commit: config.linked.before_commit.clone(),
            };
            let outbox = Arc::new(RuntimeOutbox {
                shared: shared_slot.clone(),
                node: node.clone(),
                presence: presence.clone(),
            });
            let sync = SyncService::start(cfg, outbox, sync_events)?;
            receiver.set_placer(sync.clone());
            let forward = events.clone();
            tasks.push(tokio::spawn(async move {
                loop {
                    match sync_rx.recv().await {
                        Ok(event) => {
                            let support = matches!(event, SyncEvent::Support { .. });
                            let _ = forward.send(Event::Sync(event));
                            if support {
                                let _ = forward.send(Event::DevicesChanged);
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {}
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }));
            Some(sync)
        } else {
            None
        };

        let mut router = Router::with_session(identity.name(), |_, _| {});
        router.add(CONTROL_KIND, receiver.clone())?;
        router.add(CHUNK_KIND, receiver.clone())?;
        let fling_events = events.clone();
        router.add(
            FLING_KIND,
            Arc::new(FlingService::new(Arc::new(move |from, item| {
                let _ = fling_events.send(Event::Fling { from, item });
            }))),
        )?;
        if let Some(sync) = &sync {
            router.add(SYNC_KIND, sync.clone())?;
        }
        let router = Arc::new(router);
        let shared = Arc::new(Shared {
            trust: Mutex::new(trust),
            reached: Mutex::default(),
            conns: Mutex::default(),
            router,
            events: events.clone(),
            sync: sync.clone(),
        });
        let _ = shared_slot.set(Arc::downgrade(&shared));

        let accept_task = {
            let (node, shared) = (node.clone(), shared.clone());
            tokio::spawn(async move {
                while let Some(conn) = node.accept().await {
                    match conn {
                        // A node accepts and connects from the same port, so the
                        // remote address is where this device can reach it too.
                        Ok(conn) => shared.adopt(conn),
                        Err(e) => tracing::info!("refused a connection: {e}"),
                    }
                }
            })
        };
        let probe_task = {
            let (node, shared, presence) = (node.clone(), shared.clone(), presence.clone());
            tokio::spawn(async move {
                loop {
                    probe(&node, &shared, presence.as_deref()).await;
                    tokio::time::sleep(PROBE_INTERVAL).await;
                }
            })
        };

        Ok(Arc::new(Self {
            identity,
            shared,
            node,
            presence,
            receiver,
            sync,
            home: config.home,
            default_receive_dir: config.receive_dir,
            settings: Mutex::new(settings),
            tasks: {
                tasks.extend([accept_task, probe_task]);
                tasks
            },
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

    /// The UDP port this device listens on.
    pub fn port(&self) -> u16 {
        self.node.local_addr().map(|a| a.port()).unwrap_or(0)
    }

    /// The folder of the identity, the trust store and the linked files journal.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The folder received files are saved to.
    pub fn receive_dir(&self) -> PathBuf {
        self.receiver.dir()
    }

    /// Whether received files go to the default folder.
    pub fn receive_dir_is_default(&self) -> bool {
        self.lock_settings().receive_dir.is_none()
    }

    /// Saves received files to `dir` from the next transfer on, and remembers the
    /// choice. `None` goes back to the default folder. The folder is created if needed.
    pub fn set_receive_dir(&self, dir: Option<PathBuf>) -> Result<PathBuf> {
        let target = dir
            .clone()
            .unwrap_or_else(|| self.default_receive_dir.clone());
        prepare_dir(&target)?;
        let mut settings = self.lock_settings();
        let mut updated = settings.clone();
        updated.receive_dir = dir;
        updated.save(&self.home)?;
        *settings = updated;
        self.receiver.set_dir(target.clone());
        if let Some(sync) = &self.sync {
            sync.set_linked_root(target.join(LINKED_DIR));
        }
        Ok(target)
    }

    fn lock_settings(&self) -> std::sync::MutexGuard<'_, Settings> {
        self.settings.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Subscribes to [`Event`]s.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.shared.events.subscribe()
    }

    fn seen(&self, id: &DeviceId) -> Option<DiscoveredDevice> {
        self.presence.as_ref().and_then(|p| p.get(id))
    }

    /// Paired devices, with their online state.
    pub fn devices(&self) -> Vec<DeviceStatus> {
        let paired: Vec<TrustedDevice> = self.shared.trust().devices().cloned().collect();
        paired
            .into_iter()
            .map(|device| {
                let seen = self.seen(&device.id);
                let conn = self.shared.live_conn(&device.id);
                let reached = self.shared.reached_addr(&device.id);
                let addrs = match (&seen, &conn, reached) {
                    (Some(s), _, _) => s.addrs.clone(),
                    (None, Some(c), _) => vec![c.remote_addr()],
                    (None, None, Some(addr)) => vec![addr],
                    (None, None, None) => Vec::new(),
                };
                DeviceStatus {
                    id: device.id,
                    name: seen.map_or(device.name, |s| s.name),
                    online: !addrs.is_empty(),
                    addrs,
                    linked_files: self
                        .sync
                        .as_ref()
                        .and_then(|s| s.supports(&device.id))
                        .unwrap_or(false),
                }
            })
            .collect()
    }

    /// Devices on the network that are not paired with this one (mDNS only).
    pub fn nearby_unpaired(&self) -> Vec<DiscoveredDevice> {
        let Some(presence) = &self.presence else {
            return Vec::new();
        };
        let trust = self.shared.trust();
        presence
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
            self.port(),
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
        let paired = pair_with(self.identity.clone(), &invite, self.port(), confirm).await?;
        self.remember(paired)
    }

    fn remember(&self, paired: PairedDevice) -> Result<TrustedDevice> {
        let mut device = self.shared.trust().add(paired.id, &paired.name)?;
        if let Some(addr) = paired.addr {
            self.shared.trust().set_addrs(&device.id, &[addr])?;
            device.addrs = vec![addr];
        }
        self.node.trust(device.id);
        let _ = self.shared.events.send(Event::Paired {
            id: device.id,
            name: device.name.clone(),
        });
        let _ = self.shared.events.send(Event::DevicesChanged);

        // Check the new device right away instead of at the next probe.
        let (node, shared, presence) = (
            self.node.clone(),
            self.shared.clone(),
            self.presence.clone(),
        );
        tokio::spawn(async move { probe(&node, &shared, presence.as_deref()).await });
        Ok(device)
    }

    /// Removes a paired device. Its connections are refused from now on.
    pub fn forget(&self, id: &DeviceId) -> Result<bool> {
        let removed = self.shared.trust().remove(id)?;
        self.node.untrust(id);
        if let Some(sync) = &self.sync {
            sync.forget_peer(id)?;
        }
        let conn = self
            .shared
            .conns
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        if let Some(conn) = conn {
            conn.close();
        }
        self.shared
            .reached
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        let _ = self.shared.events.send(Event::DevicesChanged);
        Ok(removed)
    }

    /// Every address worth trying for `id`: seen by mDNS, reached directly, remembered.
    fn candidate_addrs(&self, id: &DeviceId) -> Vec<SocketAddr> {
        candidate_addrs(&self.shared, self.presence.as_deref(), id)
    }

    /// Sends files and folders to a paired device.
    pub async fn send(
        &self,
        id: DeviceId,
        paths: Vec<PathBuf>,
        open: Option<OpenHint>,
        events: SendEvents,
    ) -> Result<SendReport> {
        if !self.shared.trust().contains(&id) {
            return Err(Error::NotPaired);
        }
        // A path listed twice (a share can offer the same file more than once) is
        // sent once, rather than arriving as "name (1)".
        let mut seen = std::collections::HashSet::new();
        let paths: Vec<PathBuf> = paths
            .into_iter()
            .filter(|p| seen.insert(p.clone()))
            .collect();
        let addrs = self.candidate_addrs(&id);
        if addrs.is_empty() && self.shared.live_conn(&id).is_none() {
            return Err(Error::Offline);
        }
        let (shared, presence) = (self.shared.clone(), self.presence.clone());
        let rediscover: ec_transfer::AddrSource = Arc::new(move || {
            let addrs = candidate_addrs(&shared, presence.as_deref(), &id);
            Box::pin(async move { addrs })
        });
        let shared = self.shared.clone();
        let options = SendOptions {
            rediscover: Some(rediscover),
            reuse: Some(Arc::new(move || shared.live_conn(&id))),
            open,
            ..SendOptions::default()
        };
        Ok(send_paths(self.node.clone(), id, addrs, paths, options, events).await?)
    }

    /// Flings a web page to a paired device, which offers to open it in its browser.
    pub async fn fling_url(&self, id: DeviceId, url: String, title: String) -> Result<()> {
        self.fling(id, FlingItem::Url { url, title }).await
    }

    /// Flings text to a paired device, which offers to copy it.
    pub async fn fling_text(&self, id: DeviceId, text: String) -> Result<()> {
        self.fling(id, FlingItem::Text { text }).await
    }

    async fn fling(&self, id: DeviceId, item: FlingItem) -> Result<()> {
        if !self.shared.trust().contains(&id) {
            return Err(Error::NotPaired);
        }
        let conn = match self.shared.live_conn(&id) {
            Some(conn) => conn,
            None => {
                let addrs = self.candidate_addrs(&id);
                if addrs.is_empty() {
                    return Err(Error::Offline);
                }
                let conn = self.node.connect(id, &addrs).await?;
                self.shared.adopt(conn.clone());
                conn
            }
        };
        ec_fling::fling(&conn, item).await?;
        Ok(())
    }

    fn sync(&self) -> Result<&Arc<SyncService>> {
        self.sync.as_ref().ok_or(Error::LinkedFilesOff)
    }

    /// Links the file at `path` with a paired device and sends it. On Android pass
    /// `copy_into_linked`: the file is copied into the Linked folder first and that copy
    /// is linked. Linking a file that is already linked adds the device.
    pub async fn send_linked(
        &self,
        id: DeviceId,
        path: PathBuf,
        copy_into_linked: bool,
    ) -> Result<LinkId> {
        if !self.shared.trust().contains(&id) {
            return Err(Error::NotPaired);
        }
        Ok(self.sync()?.link(id, path, copy_into_linked).await?)
    }

    /// Every linked file.
    pub fn linked_files(&self) -> Vec<LinkedFile> {
        self.sync.as_ref().map(|s| s.files()).unwrap_or_default()
    }

    /// The change log of a linked file, oldest first.
    pub fn linked_log(&self, id: LinkId) -> Result<Vec<LogEntry>> {
        Ok(self.sync()?.log(&id)?)
    }

    /// Where the waiting version of a linked file in conflict is.
    pub fn linked_conflict_path(&self, id: LinkId) -> Option<PathBuf> {
        self.sync.as_ref().and_then(|s| s.conflict_path(&id))
    }

    /// Resolves a conflict, keeping the other device's version or this one's.
    pub fn resolve_conflict(&self, id: LinkId, keep_theirs: bool) -> Result<()> {
        Ok(self.sync()?.resolve(id, keep_theirs)?)
    }

    /// Deletes the waiting version of a conflict, keeping this device's.
    pub fn delete_conflict(&self, id: LinkId) -> Result<()> {
        Ok(self.sync()?.delete_conflict(id)?)
    }

    /// Stops syncing a file on this device. The file stays.
    pub fn leave_link(&self, id: LinkId) -> Result<()> {
        Ok(self.sync()?.leave(id)?)
    }

    /// Records where a linked file is now (the app found it).
    pub fn relocate_linked(&self, id: LinkId, path: PathBuf) -> Result<()> {
        Ok(self.sync()?.relocate(id, path)?)
    }

    /// Checks every linked file now and exchanges summaries with connected devices.
    pub fn refresh_linked(&self) {
        if let Some(sync) = &self.sync {
            sync.refresh();
        }
    }

    /// The devices that reported a linked file missing.
    pub fn linked_missing_on(&self, id: LinkId) -> Vec<DeviceId> {
        self.sync
            .as_ref()
            .map(|s| s.missing_on(&id))
            .unwrap_or_default()
    }

    /// How often linked files are checked without a file system event.
    pub fn set_linked_check_interval(&self, interval: Duration) {
        if let Some(sync) = &self.sync {
            sync.set_stat_interval(interval);
        }
    }

    /// Stops listening and announcing.
    pub async fn shutdown(&self) {
        for task in &self.tasks {
            task.abort();
        }
        self.node.shutdown().await;
    }
}

fn bind_node(identity: &Arc<DeviceIdentity>, trust: &TrustStore, port: u16) -> Result<Node> {
    let trusted = || trust.devices().map(|d| d.id);
    let any = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0));
    if port != 0 {
        match Node::bind(
            identity.clone(),
            trusted(),
            SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)),
        ) {
            Ok(node) => return Ok(node),
            Err(e) => tracing::info!("port {port} unavailable ({e}); using a free port"),
        }
    }
    Ok(Node::bind(identity.clone(), trusted(), any)?)
}

fn candidate_addrs(shared: &Shared, presence: Option<&Presence>, id: &DeviceId) -> Vec<SocketAddr> {
    let mut addrs: Vec<SocketAddr> = presence
        .and_then(|p| p.get(id))
        .map(|d| d.addrs)
        .unwrap_or_default();
    addrs.extend(shared.reached_addr(id));
    if let Some(device) = shared.trust().get(id) {
        addrs.extend(device.addrs.iter().copied());
    }
    let mut seen = std::collections::HashSet::new();
    addrs.retain(|a| seen.insert(*a));
    addrs
}

/// Keeps a connection open to every paired device that can be reached, through the
/// addresses mDNS reports or the remembered ones. An open connection is what makes a
/// device show as online, and either side can send over it.
async fn probe(node: &Node, shared: &Arc<Shared>, presence: Option<&Presence>) {
    let devices: Vec<TrustedDevice> = shared.trust().devices().cloned().collect();
    for device in devices {
        if let Some(seen) = presence.and_then(|p| p.get(&device.id)) {
            let _ = shared.trust().set_addrs(&device.id, &seen.addrs);
        }
        if shared.live_conn(&device.id).is_some() {
            continue;
        }
        let addrs = candidate_addrs(shared, presence, &device.id);
        if addrs.is_empty() {
            continue;
        }
        match tokio::time::timeout(PROBE_TIMEOUT, node.connect(device.id, &addrs)).await {
            Ok(Ok(conn)) => shared.adopt(conn),
            _ => {
                let was_online = shared
                    .reached
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&device.id)
                    .is_some();
                if was_online {
                    let _ = shared.events.send(Event::DevicesChanged);
                }
            }
        }
    }
}

/// How the linked files service reaches other devices.
struct RuntimeOutbox {
    shared: Arc<OnceLock<Weak<Shared>>>,
    node: Arc<Node>,
    presence: Option<Arc<Presence>>,
}

impl RuntimeOutbox {
    fn shared(&self) -> Option<Arc<Shared>> {
        self.shared.get().and_then(Weak::upgrade)
    }
}

impl ec_sync::Outbox for RuntimeOutbox {
    fn send_file(
        &self,
        peer: DeviceId,
        path: PathBuf,
        key: LinkId,
    ) -> ec_sync::service::BoxFuture<ec_sync::Result<()>> {
        let shared = self.shared();
        let (node, presence) = (self.node.clone(), self.presence.clone());
        Box::pin(async move {
            let shared = shared.ok_or(ec_sync::Error::Offline)?;
            let addrs = candidate_addrs(&shared, presence.as_deref(), &peer);
            let reuse = shared.clone();
            let rediscover_shared = shared.clone();
            let rediscover: ec_transfer::AddrSource = Arc::new(move || {
                let addrs = candidate_addrs(&rediscover_shared, presence.as_deref(), &peer);
                Box::pin(async move { addrs })
            });
            let options = SendOptions {
                rediscover: Some(rediscover),
                reuse: Some(Arc::new(move || reuse.live_conn(&peer))),
                target: Some(key),
                ..SendOptions::default()
            };
            send_paths(node, peer, addrs, vec![path], options, Arc::new(|_| {}))
                .await
                .map(|_| ())
                .map_err(|e| ec_sync::Error::Send(e.to_string()))
        })
    }

    fn conn(&self, peer: &DeviceId) -> Option<PeerConnection> {
        self.shared()?.live_conn(peer)
    }
}
