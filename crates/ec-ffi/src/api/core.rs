//! A running device, as seen from Dart.

use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

use ec_core::DeviceId;
use ec_runtime::{Config, Event, Runtime};
use ec_transfer::{ReceiveEvent, SendEvent};
use flutter_rust_bridge::frb;
use tokio::sync::oneshot;

use crate::frb_generated::StreamSink;

/// The runtime all Rust work runs on, independent of Dart's event loop.
static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("ec-core")
        .build()
        .unwrap_or_else(|e| panic!("cannot start the async runtime: {e}"))
});

/// The core started in this process, with its home folder.
static RUNNING: Mutex<Option<(String, Arc<Runtime>)>> = Mutex::new(None);

/// Held while a core starts, so concurrent starts share one core.
static STARTING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn running(home: &str) -> Option<Arc<Runtime>> {
    let guard = RUNNING.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .as_ref()
        .filter(|(h, _)| h == home)
        .map(|(_, r)| r.clone())
}

/// Sets up logging and panic messages for Dart. Called automatically by the bridge.
#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Where the core keeps its data.
pub struct CoreConfig {
    /// Folder for the identity and paired devices (app-private).
    pub home: String,
    /// Name for this device when it is set up for the first time.
    pub device_name: String,
    /// Folder for received files.
    pub receive_dir: String,
    /// Whether the platform has a Trash for files the user chose not to keep (macOS).
    /// They are then set aside for [`EcCore::take_pending_trash`]; otherwise deleted.
    pub use_system_trash: bool,
}

/// The folder in `home` where files wait to be moved to the system Trash.
const PENDING_TRASH: &str = "trash-pending";

/// A paired device.
pub struct DeviceView {
    /// Full id (64 hex characters).
    pub id: String,
    /// Short id for display, such as `3F9A-C210`.
    pub short_id: String,
    /// Its name.
    pub name: String,
    /// Whether it is on the network now.
    pub online: bool,
    /// Whether files can be linked with it.
    pub linked_files: bool,
}

/// Where a linked file stands on this device.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LinkedState {
    /// Up to date.
    Current,
    /// A newer version is arriving.
    Updating,
    /// Changed on two devices; the user chooses.
    Conflict,
    /// Cannot be found; the link is kept.
    Missing,
}

/// The two versions of a file in conflict.
pub struct ConflictView {
    /// This device's name.
    pub mine_device: String,
    /// When this device's version was last changed (ms since the Unix epoch).
    pub mine_at_ms: u64,
    /// Its size.
    pub mine_size: u64,
    /// The other device's name.
    pub theirs_device: String,
    /// When the other version arrived (ms since the Unix epoch).
    pub theirs_at_ms: u64,
    /// Its size.
    pub theirs_size: u64,
    /// Where the other version waits, to open it.
    pub theirs_path: String,
}

/// A linked file.
pub struct LinkedView {
    /// The link id (32 hex characters).
    pub id: String,
    /// The file name on this device.
    pub name: String,
    /// Where it is on this device.
    pub path: String,
    /// Names of the devices it is linked with.
    pub devices: Vec<String>,
    /// Where it stands.
    pub state: LinkedState,
    /// Its size.
    pub size: u64,
    /// Set when in conflict.
    pub conflict: Option<ConflictView>,
}

/// One entry of a linked file's log.
pub struct LinkedLogView {
    /// When (ms since the Unix epoch).
    pub at_ms: u64,
    /// The device it happened on.
    pub device: String,
    /// What happened, for display.
    pub text: String,
}

/// What happened to a linked file.
#[derive(Clone)]
pub enum LinkedEventKind {
    /// It was linked from another device and arrived at `path`.
    Arrived {
        /// Where it was saved.
        path: String,
        /// The sending device's name.
        from: String,
    },
    /// A newer version replaced it.
    Updated,
    /// Changed on two devices; the user chooses.
    Conflict,
    /// The conflict was resolved.
    Resolved,
    /// Another device left the link.
    PeerLeft {
        /// Its name.
        device: String,
    },
    /// This device left the link.
    Left,
    /// It cannot be found.
    Missing,
    /// It was found again.
    Found,
    /// It was moved or renamed here.
    Moved,
    /// Something went wrong.
    Error {
        /// What.
        message: String,
    },
}

/// Background events for the UI.
#[derive(Clone)]
pub enum CoreEvent {
    /// Read `devices()` again.
    DevicesChanged,
    /// A device was paired.
    Paired {
        /// Its name.
        name: String,
    },
    /// Files are arriving.
    ReceiveStarted {
        /// Short id of the sender.
        from: String,
        /// Number of files.
        files: u32,
        /// Total size.
        total_bytes: u64,
        /// Bytes already here from an interrupted attempt.
        already_have: u64,
    },
    /// Receive progress.
    ReceiveProgress {
        /// Bytes received.
        received: u64,
        /// Total bytes.
        total: u64,
    },
    /// A file was saved.
    Saved {
        /// Its path.
        path: String,
        /// The sender flung it: open it now.
        open: bool,
        /// The page to open at, for documents.
        page: Option<u32>,
    },
    /// A paired device flung a web page to open here.
    FlingUrl {
        /// Short id of the sender.
        from: String,
        /// The address (validated: http or https).
        url: String,
        /// The page title.
        title: String,
    },
    /// A paired device flung text, to offer to copy.
    FlingText {
        /// Short id of the sender.
        from: String,
        /// The text (validated: no control characters but line breaks and tabs).
        text: String,
    },
    /// A transfer finished.
    ReceiveFinished,
    /// A transfer stopped early; it resumes when sent again.
    ReceiveInterrupted {
        /// Why.
        reason: String,
    },
    /// Something happened to a linked file; read `linked_files()` again.
    Linked {
        /// The link id.
        id: String,
        /// What happened.
        kind: LinkedEventKind,
    },
}

/// Steps of pairing.
#[derive(Clone)]
pub enum PairingEvent {
    /// Show this invite as a QR code (when hosting).
    Invite {
        /// The `ecpair:` link.
        uri: String,
    },
    /// Ask the user to compare the code, then call `answer_pairing`.
    Confirm {
        /// The other device's name.
        peer_name: String,
        /// Its short id.
        short_id: String,
        /// The six-digit code, as `"042 917"`.
        code: String,
    },
    /// Pairing finished.
    Paired {
        /// The other device's name.
        name: String,
    },
    /// Pairing failed.
    Failed {
        /// Why.
        reason: String,
    },
}

/// Progress of a send.
#[derive(Clone)]
pub enum SendUpdate {
    /// Bytes acknowledged by the receiver.
    Progress {
        /// Sent so far.
        sent: u64,
        /// To send in this attempt.
        to_send: u64,
        /// Network paths in use.
        paths: u32,
    },
    /// A connection was lost and the send is retrying.
    Retrying {
        /// Why.
        reason: String,
    },
    /// Finished.
    Done {
        /// Number of files.
        files: u32,
        /// Bytes sent.
        bytes: u64,
        /// Duration in milliseconds.
        millis: u64,
        /// Paths used.
        paths: u32,
    },
    /// Failed.
    Failed {
        /// Why.
        reason: String,
    },
}

/// A running device. Create it once with [`EcCore::start`].
pub struct EcCore {
    runtime: Arc<Runtime>,
    pending_answer: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
}

impl EcCore {
    /// Starts the device: identity, network presence and receiving.
    ///
    /// The core outlives the Dart side: on Android the activity (and its Dart isolate)
    /// can be recreated while the process keeps receiving in the background. Starting
    /// again with the same home returns the running core instead of a second one.
    pub async fn start(config: CoreConfig) -> anyhow::Result<EcCore> {
        // Two Flutter engines (the app and the Android fling layer) may start at the
        // same moment; the lock makes the second one wait and join the first core
        // instead of starting another with its own, stale copy of the trust store.
        let _starting = STARTING.lock().await;
        if let Some(runtime) = running(&config.home) {
            return Ok(Self::wrap(runtime));
        }
        let home = config.home.clone();
        let pending_trash = PathBuf::from(&config.home).join(PENDING_TRASH);
        let trash: ec_runtime::TrashFn = if config.use_system_trash {
            // Set aside; the app moves it to the Trash (see `take_pending_trash`).
            Arc::new(move |path: &std::path::Path| {
                std::fs::create_dir_all(&pending_trash)?;
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let mut to = pending_trash.join(&*name);
                let mut n = 1;
                while to.exists() {
                    to = pending_trash.join(format!("{n} {name}"));
                    n += 1;
                }
                match std::fs::rename(path, &to) {
                    // On another volume (a receive folder on an external disk).
                    Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => {
                        std::fs::copy(path, &to)?;
                        std::fs::remove_file(path)
                    }
                    other => other,
                }
            })
        } else {
            Arc::new(|path: &std::path::Path| std::fs::remove_file(path))
        };
        let config = Config {
            home: PathBuf::from(config.home),
            default_name: config.device_name,
            receive_dir: PathBuf::from(config.receive_dir),
            port: ec_runtime::DEFAULT_PORT,
            mdns: true,
            linked: ec_runtime::LinkedConfig {
                trash: Some(trash),
                ..Default::default()
            },
        };
        let runtime = RT.spawn(Runtime::start(config)).await??;
        *RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = Some((home, runtime.clone()));
        Ok(Self::wrap(runtime))
    }

    fn wrap(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            pending_answer: Arc::default(),
        }
    }

    /// This device's full id.
    #[frb(sync)]
    pub fn my_id(&self) -> String {
        self.runtime.id().to_string()
    }

    /// This device's short id.
    #[frb(sync)]
    pub fn my_short_id(&self) -> String {
        self.runtime.id().short()
    }

    /// This device's name.
    #[frb(sync)]
    pub fn my_name(&self) -> String {
        self.runtime.name().to_owned()
    }

    /// The folder received files are saved to.
    #[frb(sync)]
    pub fn receive_dir(&self) -> String {
        self.runtime.receive_dir().to_string_lossy().into_owned()
    }

    /// Whether received files go to the app's default folder.
    #[frb(sync)]
    pub fn receive_dir_is_default(&self) -> bool {
        self.runtime.receive_dir_is_default()
    }

    /// Saves received files to `path` from the next transfer on; `None` goes back to
    /// the default folder. Returns the folder now in use.
    #[frb(sync)]
    pub fn set_receive_dir(&self, path: Option<String>) -> anyhow::Result<String> {
        let dir = self.runtime.set_receive_dir(path.map(PathBuf::from))?;
        Ok(dir.to_string_lossy().into_owned())
    }

    /// Paired devices.
    #[frb(sync)]
    pub fn devices(&self) -> Vec<DeviceView> {
        self.runtime
            .devices()
            .into_iter()
            .map(|d| DeviceView {
                id: d.id.to_string(),
                short_id: d.id.short(),
                name: d.name,
                online: d.online,
                linked_files: d.linked_files,
            })
            .collect()
    }

    fn device_name(&self, id: &DeviceId) -> String {
        if *id == self.runtime.id() {
            return self.runtime.name().to_owned();
        }
        self.runtime
            .devices()
            .into_iter()
            .find(|d| d.id == *id)
            .map_or_else(|| id.short(), |d| d.name)
    }

    /// Links a file with a paired device and sends it. On Android pass
    /// `copy_into_linked`: the shared file is copied into the Linked folder first.
    /// Returns the link id.
    pub async fn send_linked(
        &self,
        device_id: String,
        path: String,
        copy_into_linked: bool,
    ) -> anyhow::Result<String> {
        let id: DeviceId = device_id.parse()?;
        let runtime = self.runtime.clone();
        let link = RT
            .spawn(async move {
                runtime
                    .send_linked(id, PathBuf::from(path), copy_into_linked)
                    .await
            })
            .await??;
        Ok(hex::encode(link))
    }

    /// Every linked file.
    #[frb(sync)]
    pub fn linked_files(&self) -> Vec<LinkedView> {
        self.runtime
            .linked_files()
            .into_iter()
            .map(|f| {
                let (state, conflict) = match &f.state {
                    ec_sync::State::Current => (LinkedState::Current, None),
                    ec_sync::State::Updating => (LinkedState::Updating, None),
                    ec_sync::State::Missing => (LinkedState::Missing, None),
                    ec_sync::State::Conflict {
                        theirs,
                        from,
                        at_ms,
                        ..
                    } => {
                        let view = ConflictView {
                            mine_device: self.runtime.name().to_owned(),
                            mine_at_ms: (f.seen.mtime_ns / 1_000_000).max(0) as u64,
                            mine_size: f.content.size,
                            theirs_device: self.device_name(from),
                            theirs_at_ms: *at_ms,
                            theirs_size: theirs.size,
                            theirs_path: self
                                .runtime
                                .linked_conflict_path(f.id)
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        };
                        (LinkedState::Conflict, Some(view))
                    }
                };
                LinkedView {
                    id: hex::encode(f.id),
                    name: f.name.clone(),
                    path: f.path.to_string_lossy().into_owned(),
                    devices: f.peers.iter().map(|p| self.device_name(p)).collect(),
                    state,
                    size: f.content.size,
                    conflict,
                }
            })
            .collect()
    }

    /// The log of a linked file, newest first.
    #[frb(sync)]
    pub fn linked_log(&self, id: String) -> anyhow::Result<Vec<LinkedLogView>> {
        let log = self.runtime.linked_log(link_id(&id)?)?;
        Ok(log
            .into_iter()
            .rev()
            .map(|entry| LinkedLogView {
                at_ms: entry.at_ms,
                device: self.device_name(&entry.by),
                text: log_text(&entry.kind, |d| self.device_name(d)),
            })
            .collect())
    }

    /// Resolves a conflict, keeping the other device's version or this one's.
    #[frb(sync)]
    pub fn resolve_conflict(&self, id: String, keep_theirs: bool) -> anyhow::Result<()> {
        Ok(self.runtime.resolve_conflict(link_id(&id)?, keep_theirs)?)
    }

    /// Deletes the waiting version of a conflict, keeping this device's.
    #[frb(sync)]
    pub fn delete_conflict(&self, id: String) -> anyhow::Result<()> {
        Ok(self.runtime.delete_conflict(link_id(&id)?)?)
    }

    /// Stops syncing a file on this device. The file stays.
    #[frb(sync)]
    pub fn leave_link(&self, id: String) -> anyhow::Result<()> {
        Ok(self.runtime.leave_link(link_id(&id)?)?)
    }

    /// Records where a linked file is now (found by the app, for example through a
    /// bookmark). A path in the Trash means it was deleted.
    #[frb(sync)]
    pub fn relocate_linked(&self, id: String, path: String) -> anyhow::Result<()> {
        Ok(self
            .runtime
            .relocate_linked(link_id(&id)?, PathBuf::from(path))?)
    }

    /// How often linked files are checked without a file system event, in seconds.
    #[frb(sync)]
    pub fn set_linked_check_interval(&self, seconds: u32) {
        self.runtime
            .set_linked_check_interval(std::time::Duration::from_secs(u64::from(seconds.max(1))));
    }

    /// Files set aside for the system Trash (with `use_system_trash`). The app moves
    /// each to the Trash; they stay listed until it does.
    #[frb(sync)]
    pub fn take_pending_trash(&self) -> Vec<String> {
        let dir = self.runtime.home().join(PENDING_TRASH);
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Streams background events until the stream is closed on the Dart side.
    pub fn events(&self, sink: StreamSink<CoreEvent>) {
        let mut events = self.runtime.subscribe();
        RT.spawn(async move {
            loop {
                let event = match events.recv().await {
                    Ok(event) => event,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => return,
                };
                if let Some(event) = convert(event)
                    && sink.add(event).is_err()
                {
                    return;
                }
            }
        });
    }

    /// Hosts pairing: emits an invite to show, then asks for confirmation.
    pub fn host_pairing(&self, sink: StreamSink<PairingEvent>) {
        let runtime = self.runtime.clone();
        let pending = self.pending_answer.clone();
        RT.spawn(async move {
            let invite_sink = sink.clone();
            let result = runtime
                .host_pairing(
                    move |uri| {
                        let _ = invite_sink.add(PairingEvent::Invite { uri });
                    },
                    confirm_through(sink.clone(), pending),
                )
                .await;
            finish_pairing(&sink, result.map(|d| d.name));
        });
    }

    /// Joins the device that shows `invite`.
    pub fn join_pairing(&self, invite: String, sink: StreamSink<PairingEvent>) {
        let runtime = self.runtime.clone();
        let pending = self.pending_answer.clone();
        RT.spawn(async move {
            let result = runtime
                .join_pairing(&invite, confirm_through(sink.clone(), pending))
                .await;
            finish_pairing(&sink, result.map(|d| d.name));
        });
    }

    /// The user's answer to the last [`PairingEvent::Confirm`].
    #[frb(sync)]
    pub fn answer_pairing(&self, accept: bool) {
        let sender = self
            .pending_answer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(sender) = sender {
            let _ = sender.send(accept);
        }
    }

    /// Sends files and folders to a paired device. With `open`, it opens them on
    /// arrival (a fling), at `page` if given.
    pub fn send(
        &self,
        device_id: String,
        paths: Vec<String>,
        open: bool,
        page: Option<u32>,
        sink: StreamSink<SendUpdate>,
    ) {
        let runtime = self.runtime.clone();
        RT.spawn(async move {
            let id = match device_id.parse::<DeviceId>() {
                Ok(id) => id,
                Err(e) => {
                    let _ = sink.add(SendUpdate::Failed {
                        reason: e.to_string(),
                    });
                    return;
                }
            };
            let progress = sink.clone();
            let events: ec_transfer::SendEvents = Arc::new(move |event| {
                let update = match event {
                    SendEvent::Progress {
                        sent,
                        to_send,
                        paths,
                    } => SendUpdate::Progress {
                        sent,
                        to_send,
                        paths: paths as u32,
                    },
                    SendEvent::Retrying { reason, .. } => SendUpdate::Retrying { reason },
                    _ => return,
                };
                let _ = progress.add(update);
            });
            let paths = paths.into_iter().map(PathBuf::from).collect();
            let hint = open.then_some(ec_transfer::OpenHint { page });
            let update = match runtime.send(id, paths, hint, events).await {
                Ok(report) => SendUpdate::Done {
                    files: report.files as u32,
                    bytes: report.bytes_sent,
                    millis: report.elapsed.as_millis() as u64,
                    paths: report.paths_used as u32,
                },
                Err(e) => SendUpdate::Failed {
                    reason: e.to_string(),
                },
            };
            let _ = sink.add(update);
        });
    }

    /// Flings a web page to a paired device, which offers to open it in its browser.
    pub async fn fling_url(
        &self,
        device_id: String,
        url: String,
        title: String,
    ) -> anyhow::Result<()> {
        let id: DeviceId = device_id.parse()?;
        let runtime = self.runtime.clone();
        RT.spawn(async move { runtime.fling_url(id, url, title).await })
            .await??;
        Ok(())
    }

    /// Flings text to a paired device, which offers to copy it.
    pub async fn fling_text(&self, device_id: String, text: String) -> anyhow::Result<()> {
        let id: DeviceId = device_id.parse()?;
        let runtime = self.runtime.clone();
        RT.spawn(async move { runtime.fling_text(id, text).await })
            .await??;
        Ok(())
    }

    /// Removes a paired device.
    #[frb(sync)]
    pub fn forget(&self, device_id: String) -> anyhow::Result<()> {
        let id: DeviceId = device_id.parse()?;
        self.runtime.forget(&id)?;
        Ok(())
    }
}

/// A pairing confirmation that asks Dart and waits for [`EcCore::answer_pairing`].
fn confirm_through(
    sink: StreamSink<PairingEvent>,
    pending: Arc<Mutex<Option<oneshot::Sender<bool>>>>,
) -> impl FnOnce(ec_net::pairing::PairingPrompt) -> std::pin::Pin<Box<dyn Future<Output = bool> + Send>>
{
    move |prompt| {
        let (tx, rx) = oneshot::channel();
        *pending.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        let _ = sink.add(PairingEvent::Confirm {
            peer_name: prompt.peer_name,
            short_id: prompt.peer.short(),
            code: prompt.code,
        });
        Box::pin(async move { rx.await.unwrap_or(false) })
    }
}

fn finish_pairing(sink: &StreamSink<PairingEvent>, result: ec_runtime::Result<String>) {
    let event = match result {
        Ok(name) => PairingEvent::Paired { name },
        Err(e) => PairingEvent::Failed {
            reason: e.to_string(),
        },
    };
    let _ = sink.add(event);
}

fn link_id(id: &str) -> anyhow::Result<ec_sync::LinkId> {
    let bytes = hex::decode(id)?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("not a link id: {id}"))
}

/// A log entry, for display.
fn log_text(kind: &ec_sync::LogKind, name: impl Fn(&DeviceId) -> String) -> String {
    use ec_sync::LogKind;
    match kind {
        LogKind::Linked => "linked".into(),
        LogKind::Changed => "changed".into(),
        LogKind::Received => "received a new version".into(),
        LogKind::Moved { from, to } => format!("moved from {} to {}", from.display(), to.display()),
        LogKind::Conflict => "changed on two devices".into(),
        LogKind::Resolved => "conflict resolved".into(),
        LogKind::Unlinked => "left the link".into(),
        LogKind::PeerLeft(device) => format!("{} left the link", name(device)),
        LogKind::Missing => "cannot be found".into(),
        LogKind::Found => "found again".into(),
    }
}

fn convert_sync(event: ec_sync::SyncEvent) -> Option<CoreEvent> {
    use ec_sync::SyncEvent;
    let (id, kind) = match event {
        SyncEvent::Linked { id, from, path } => (
            id,
            LinkedEventKind::Arrived {
                path: path.to_string_lossy().into_owned(),
                from: from.short(),
            },
        ),
        SyncEvent::Updated { id } => (id, LinkedEventKind::Updated),
        SyncEvent::Conflict { id } => (id, LinkedEventKind::Conflict),
        SyncEvent::Resolved { id } => (id, LinkedEventKind::Resolved),
        SyncEvent::PeerUnlinked { id, peer } => (
            id,
            LinkedEventKind::PeerLeft {
                device: peer.short(),
            },
        ),
        SyncEvent::LeftLink { id } => (id, LinkedEventKind::Left),
        SyncEvent::Missing { id } => (id, LinkedEventKind::Missing),
        SyncEvent::Found { id } => (id, LinkedEventKind::Found),
        SyncEvent::Moved { id } => (id, LinkedEventKind::Moved),
        SyncEvent::Error { id, message } => (id, LinkedEventKind::Error { message }),
        // Shown through DevicesChanged.
        SyncEvent::Support { .. } => return None,
    };
    Some(CoreEvent::Linked {
        id: hex::encode(id),
        kind,
    })
}

fn convert(event: Event) -> Option<CoreEvent> {
    Some(match event {
        Event::DevicesChanged => CoreEvent::DevicesChanged,
        Event::Paired { name, .. } => CoreEvent::Paired { name },
        Event::Fling { from, item } => match item {
            ec_fling::FlingItem::Url { url, title } => CoreEvent::FlingUrl {
                from: from.short(),
                url,
                title,
            },
            ec_fling::FlingItem::Text { text } => CoreEvent::FlingText {
                from: from.short(),
                text,
            },
        },
        Event::Receive(event) => match event {
            ReceiveEvent::Started {
                peer,
                files,
                total_bytes,
                already_have,
            } => CoreEvent::ReceiveStarted {
                from: peer.short(),
                files: files as u32,
                total_bytes,
                already_have,
            },
            ReceiveEvent::Progress { received, total } => {
                CoreEvent::ReceiveProgress { received, total }
            }
            // Linked files arrive as sync events instead of an arrival card each time.
            ReceiveEvent::Saved { linked: true, .. } => return None,
            ReceiveEvent::Saved { path, open, .. } => CoreEvent::Saved {
                path: path.to_string_lossy().into_owned(),
                open: open.is_some(),
                page: open.and_then(|o| o.page),
            },
            ReceiveEvent::Finished => CoreEvent::ReceiveFinished,
            ReceiveEvent::Interrupted { reason } => CoreEvent::ReceiveInterrupted { reason },
        },
        Event::Sync(event) => return convert_sync(event),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn concurrent_starts_share_one_core() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let config = || CoreConfig {
            home: dir.path().join("home").to_string_lossy().into_owned(),
            device_name: "Tablet".into(),
            receive_dir: dir.path().join("inbox").to_string_lossy().into_owned(),
            use_system_trash: false,
        };
        let (a, b) = tokio::join!(EcCore::start(config()), EcCore::start(config()));
        let (a, b) = (
            a.unwrap_or_else(|e| panic!("{e}")),
            b.unwrap_or_else(|e| panic!("{e}")),
        );
        assert!(
            Arc::ptr_eq(&a.runtime, &b.runtime),
            "both engines got the same core"
        );
    }
}
