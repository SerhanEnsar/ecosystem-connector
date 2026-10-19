//! The linked files service: sync messages between devices, decisions, placing received
//! versions and reacting to local changes.
//!
//! Each message is one short bidirectional stream of kind [`SYNC_KIND`]: an announcement
//! (`Link`, `Changed`, `Resolved`) answered with `Reply`, an `Unlinked` notice, or a
//! `Summary` (in batches) answered with `Reply` on the last batch. An older device stops
//! the unknown stream, which is how support is detected. Contents travel as ordinary
//! transfers, started by the device that has them after a `Reply { want: true }`.

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use ec_core::DeviceId;
use ec_net::{PeerConnection, RecvStream, SendStream, Service, ServiceFuture};
use ec_transfer::{FileEntry, Placement, Placer, RelPath};
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

use crate::identity::stat;
use crate::proto::{
    ContentHeader, SUMMARY_BATCH, SYNC_KIND, SyncMessage, read_content, read_sync, send_content,
    write_sync,
};
use crate::watch::{WatchConfig, WatchEvent, Watched, Watcher, describe};
use crate::{
    Content, Error, Journal, LinkId, LinkedFile, LogEntry, LogKind, Order, Result, State,
    VersionVector, new_link_id,
};

const STEP_TIMEOUT: Duration = Duration::from_secs(10);
/// The id used in the `Reply` that acknowledges a summary.
const SUMMARY_ACK: LinkId = [0; 16];
const CONFLICTS_DIR: &str = "conflicts";

/// A future returned by an [`Outbox`].
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// How the service reaches other devices; implemented by the runtime.
pub trait Outbox: Send + Sync {
    /// Sends the file at `path` to `peer` as a new version of the linked file `key`.
    fn send_file(&self, peer: DeviceId, path: PathBuf, key: LinkId) -> BoxFuture<Result<()>>;
    /// A live connection with `peer`, if there is one.
    fn conn(&self, peer: &DeviceId) -> Option<PeerConnection>;
}

/// Moves a file to the system Trash (macOS) or deletes it (Android).
pub type TrashFn = Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>;
/// Milliseconds since the Unix epoch.
pub type ClockFn = Arc<dyn Fn() -> u64 + Send + Sync>;
/// Called with the linked file's path just before a received version replaces it.
pub type CommitHook = Arc<dyn Fn(&Path) + Send + Sync>;

/// How to run the service.
#[derive(Clone)]
pub struct SyncConfig {
    /// This device.
    pub me: DeviceId,
    /// Folder of the journal and of waiting conflict versions.
    pub dir: PathBuf,
    /// `<receive folder>/Linked`, where linked files from other devices go.
    pub linked_root: PathBuf,
    /// How a file the user chose not to keep is removed.
    pub trash: TrashFn,
    /// The clock, for log entries and the dated folders.
    pub now_ms: ClockFn,
    /// How to watch the files.
    pub watch: WatchConfig,
    /// For tests: runs just before a received version replaces the linked file.
    #[doc(hidden)]
    pub before_commit: Option<CommitHook>,
}

/// Something the user interface may want to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncEvent {
    /// A file was linked from another device and arrived.
    Linked {
        /// The link.
        id: LinkId,
        /// The device it came from.
        from: DeviceId,
        /// Where it was saved.
        path: PathBuf,
    },
    /// A newer version arrived and replaced the file.
    Updated {
        /// The link.
        id: LinkId,
    },
    /// Two devices changed the same version; the user chooses.
    Conflict {
        /// The link.
        id: LinkId,
    },
    /// A conflict was resolved.
    Resolved {
        /// The link.
        id: LinkId,
    },
    /// Another device left the link.
    PeerUnlinked {
        /// The link.
        id: LinkId,
        /// The device.
        peer: DeviceId,
    },
    /// This device left the link (the file was deleted here, or the user chose to).
    LeftLink {
        /// The link.
        id: LinkId,
    },
    /// The file cannot be found; the link is kept.
    Missing {
        /// The link.
        id: LinkId,
    },
    /// A missing file is back.
    Found {
        /// The link.
        id: LinkId,
    },
    /// The file was moved or renamed on this device.
    Moved {
        /// The link.
        id: LinkId,
    },
    /// Whether a device supports linked files, learned when it connected.
    Support {
        /// The device.
        peer: DeviceId,
        /// Whether it does.
        supported: bool,
    },
    /// Something went wrong with a file.
    Error {
        /// The link.
        id: LinkId,
        /// What.
        message: String,
    },
}

/// What to do with a version announced by another device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// This device has it or something newer.
    Ignore,
    /// It follows this device's version: fetch it.
    Update,
    /// It was made independently of this device's version: fetch it and let the user
    /// choose.
    Conflict,
    /// Same contents, a version that includes more changes: take the version only.
    AdoptVersion,
}

/// Decides what to do with version `incoming` (with chunk `hashes`) of `local`.
pub fn decide(local: &LinkedFile, incoming: &VersionVector, hashes: &[[u8; 32]]) -> Decision {
    let same = local.content.hashes == hashes;
    match local.version.compare(incoming) {
        Order::Equal | Order::After => Decision::Ignore,
        Order::Before | Order::Concurrent if same => Decision::AdoptVersion,
        Order::Before => Decision::Update,
        Order::Concurrent => Decision::Conflict,
    }
}

/// A version this device agreed to receive, waiting for its transfer.
#[derive(Clone, Debug)]
struct Pending {
    kind: Incoming,
    version: VersionVector,
    content: Content,
}

#[derive(Clone, Debug)]
enum Incoming {
    /// A file this device does not have yet.
    New { name: String },
    /// A newer version of a file it has.
    Update,
    /// A conflicting version.
    Conflict,
}

struct Core {
    cfg: SyncConfig,
    linked_root: Mutex<PathBuf>,
    journal: Mutex<Journal>,
    watcher: Watcher,
    outbox: Arc<dyn Outbox>,
    events: broadcast::Sender<SyncEvent>,
    support: Mutex<HashMap<DeviceId, bool>>,
    pending: Mutex<HashMap<(DeviceId, LinkId), Pending>>,
}

/// The linked files service. Register it with an [`ec_net::Router`] for [`SYNC_KIND`]
/// and as the [`ec_transfer::Receiver`]'s placer.
pub struct SyncService {
    core: Arc<Core>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn io_err(action: &'static str, path: &Path, source: std::io::Error) -> Error {
    Error::Io {
        action,
        path: path.to_path_buf(),
        source,
    }
}

fn watched(journal: &Journal) -> Vec<Watched> {
    journal
        .files()
        .map(|f| Watched {
            id: f.id,
            path: f.path.clone(),
            seen: f.seen,
            content: f.content.clone(),
            identity: f.identity,
        })
        .collect()
}

fn header(content: &Content) -> ContentHeader {
    ContentHeader {
        size: content.size,
        chunk_size: content.chunk_size,
    }
}

/// `YYYY-MM-DD` (UTC) of a time in milliseconds since the Unix epoch.
pub fn date_folder(ms: u64) -> String {
    // Days to a civil date, after Howard Hinnant's `civil_from_days`.
    let z = (ms / 86_400_000) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Whether `path` is inside a Trash folder, where a "moved" file has really been deleted.
fn in_trash(path: &Path) -> bool {
    path.components()
        .any(|c| matches!(c.as_os_str().to_str(), Some(".Trash" | ".Trashes")))
}

impl SyncService {
    /// Opens the journal, starts watching the linked files and returns the service.
    /// Must run inside a tokio runtime.
    pub fn start(
        cfg: SyncConfig,
        outbox: Arc<dyn Outbox>,
        events: broadcast::Sender<SyncEvent>,
    ) -> Result<Arc<Self>> {
        let journal = Journal::open(&cfg.dir)?;
        if journal.recovered() {
            tracing::warn!("linked files journal was unreadable; starting over");
        }
        let (tx, rx) = mpsc::unbounded_channel();
        let watcher = Watcher::start(cfg.watch.clone(), watched(&journal), tx)?;
        let core = Arc::new(Core {
            linked_root: Mutex::new(cfg.linked_root.clone()),
            cfg,
            journal: Mutex::new(journal),
            watcher,
            outbox,
            events,
            support: Mutex::new(HashMap::new()),
            pending: Mutex::new(HashMap::new()),
        });
        tokio::spawn(watch_loop(Arc::downgrade(&core), rx));
        Ok(Arc::new(Self { core }))
    }

    /// Links the file at `path` with `peer` and sends it. With `copy_into_linked` the
    /// file is first copied into the Linked folder and that copy is linked (Android,
    /// where a shared file's original cannot be watched). A file that is already linked
    /// gets `peer` added. If `peer` is offline, it receives the file when it connects.
    pub async fn link(
        &self,
        peer: DeviceId,
        path: PathBuf,
        copy_into_linked: bool,
    ) -> Result<LinkId> {
        self.core.link(peer, path, copy_into_linked).await
    }

    /// Every linked file.
    pub fn files(&self) -> Vec<LinkedFile> {
        lock(&self.core.journal).files().cloned().collect()
    }

    /// The change log of `id`, oldest first.
    pub fn log(&self, id: &LinkId) -> Result<Vec<LogEntry>> {
        lock(&self.core.journal).log(id)
    }

    /// Where the waiting version of a file in conflict is kept.
    pub fn conflict_path(&self, id: &LinkId) -> Option<PathBuf> {
        let journal = lock(&self.core.journal);
        let file = journal.get(id)?;
        matches!(file.state, State::Conflict { .. }).then(|| self.core.conflict_path(file))
    }

    /// Resolves a conflict: keeps the other device's version (`keep_theirs`) or this
    /// device's. The other one is moved to the Trash.
    pub fn resolve(&self, id: LinkId, keep_theirs: bool) -> Result<()> {
        self.core.resolve(id, keep_theirs)
    }

    /// Deletes the waiting version of a conflict, keeping this device's.
    pub fn delete_conflict(&self, id: LinkId) -> Result<()> {
        self.core.resolve(id, false)
    }

    /// Leaves the link. The file stays where it is and is no longer synced.
    pub fn leave(&self, id: LinkId) -> Result<()> {
        self.core.leave(id)
    }

    /// Records that the file is now at `path` (the app found it, for example by
    /// resolving a bookmark). A path in a Trash folder means it was deleted.
    pub fn relocate(&self, id: LinkId, path: PathBuf) -> Result<()> {
        if in_trash(&path) {
            return self.core.leave(id);
        }
        let identity = stat(&path).and_then(|(_, identity)| identity);
        self.core.moved(id, path, identity);
        Ok(())
    }

    /// Tells the service that `peer` connected: they exchange summaries, which also
    /// shows whether `peer` supports linked files.
    pub fn connected(&self, peer: DeviceId) {
        let core = self.core.clone();
        tokio::spawn(async move { core.send_summary(peer).await });
    }

    /// Whether `peer` supports linked files, once known.
    pub fn supports(&self, peer: &DeviceId) -> Option<bool> {
        lock(&self.core.support).get(peer).copied()
    }

    /// Removes an unpaired device from every link.
    pub fn forget_peer(&self, peer: &DeviceId) -> Result<()> {
        lock(&self.core.support).remove(peer);
        lock(&self.core.journal).remove_peer(peer)
    }

    /// Checks sizes and modification times this often from now on.
    pub fn set_stat_interval(&self, interval: Duration) {
        self.core.watcher.set_stat_interval(interval);
    }

    /// Saves linked files from other devices under `root` from now on.
    pub fn set_linked_root(&self, root: PathBuf) {
        *lock(&self.core.linked_root) = root;
    }

    #[cfg(test)]
    fn core(&self) -> &Arc<Core> {
        &self.core
    }
}

impl Core {
    fn emit(&self, event: SyncEvent) {
        let _ = self.events.send(event);
    }

    fn now(&self) -> u64 {
        (self.cfg.now_ms)()
    }

    fn log(&self, journal: &mut Journal, file: &LinkedFile, by: DeviceId, kind: LogKind) {
        let entry = LogEntry {
            at_ms: self.now(),
            by,
            kind,
            version: file.version.clone(),
            size: file.content.size,
        };
        if let Err(e) = journal.append(&file.id, entry) {
            tracing::warn!("cannot write the linked files log: {e}");
        }
    }

    /// Saves `file` and logs `kind`.
    fn save(&self, file: &LinkedFile, by: DeviceId, kind: LogKind) -> Result<()> {
        let mut journal = lock(&self.journal);
        journal.put(file.clone())?;
        self.log(&mut journal, file, by, kind);
        Ok(())
    }

    fn get(&self, id: &LinkId) -> Option<LinkedFile> {
        lock(&self.journal).get(id).cloned()
    }

    fn refresh_watch(&self) {
        let files = watched(&lock(&self.journal));
        self.watcher.set(files);
    }

    fn conflict_path(&self, file: &LinkedFile) -> PathBuf {
        self.cfg
            .dir
            .join(CONFLICTS_DIR)
            .join(format!("{}-{}", hex::encode(file.id), file.name))
    }

    async fn link(self: &Arc<Self>, peer: DeviceId, path: PathBuf, copy: bool) -> Result<LinkId> {
        let existing = lock(&self.journal)
            .files()
            .find(|f| f.path == path)
            .cloned();
        let file = match existing {
            Some(mut file) => {
                if !file.peers.contains(&peer) {
                    file.peers.push(peer);
                    lock(&self.journal).put(file.clone())?;
                }
                file
            }
            None => {
                let path = if copy {
                    self.copy_into_linked(&path)?
                } else {
                    path
                };
                let id = new_link_id();
                let described = {
                    let path = path.clone();
                    tokio::task::spawn_blocking(move || describe(id, &path))
                        .await
                        .map_err(|_| Error::Refused("hashing task failed"))??
                };
                let mut version = VersionVector::default();
                version.bump(self.cfg.me);
                let file = LinkedFile {
                    id,
                    name: file_name(&path),
                    path,
                    identity: described.identity,
                    peers: vec![peer],
                    version,
                    content: described.content,
                    seen: described.seen,
                    state: State::Current,
                };
                self.save(&file, self.cfg.me, LogKind::Linked)?;
                self.refresh_watch();
                file
            }
        };
        let msg = SyncMessage::Link {
            id: file.id,
            name: file.name.clone(),
            version: file.version.clone(),
            content: header(&file.content),
        };
        match self.announce(peer, msg, &file).await {
            // It goes out with the summary when the device connects.
            Ok(()) | Err(Error::Offline) => Ok(file.id),
            Err(e) => Err(e),
        }
    }

    fn copy_into_linked(&self, from: &Path) -> Result<PathBuf> {
        let dir = lock(&self.linked_root).join(date_folder(self.now()));
        std::fs::create_dir_all(&dir).map_err(|e| io_err("create directory", &dir, e))?;
        let rel = RelPath::new(vec![file_name(from)])?;
        let to = ec_transfer::path::unique_destination(&dir, &rel)?;
        std::fs::copy(from, &to).map_err(|e| io_err("copy", from, e))?;
        Ok(to)
    }

    /// Announces a version of `file` to `peer` and sends the contents if asked.
    async fn announce(&self, peer: DeviceId, msg: SyncMessage, file: &LinkedFile) -> Result<()> {
        let conn = self.outbox.conn(&peer).ok_or(Error::Offline)?;
        let (mut send, mut recv) = conn.open_bi(SYNC_KIND).await?;
        send_content(&mut send, &msg, &file.content.hashes).await?;
        let reply = timeout(STEP_TIMEOUT, read_sync(&mut recv))
            .await
            .map_err(|_| Error::Refused("no answer"))??;
        let _ = send.finish();
        match reply {
            SyncMessage::Reply { id, want } if id == file.id => {
                if want {
                    self.outbox
                        .send_file(peer, file.path.clone(), file.id)
                        .await?;
                }
                Ok(())
            }
            _ => Err(Error::Refused("unexpected answer")),
        }
    }

    /// Announces the current version of `id` to its peers in the background.
    fn announce_to_peers(self: &Arc<Self>, id: LinkId, resolved: bool) {
        let Some(file) = self.get(&id) else {
            return;
        };
        for peer in file.peers.clone() {
            self.announce_in_background(peer, &file, resolved);
        }
    }

    /// Passes a version received from `from` on to the file's other devices, so a
    /// device linked with several others keeps them all current.
    fn forward(self: &Arc<Self>, id: LinkId, from: DeviceId) {
        let Some(file) = self.get(&id) else {
            return;
        };
        if matches!(file.state, State::Conflict { .. }) {
            return;
        }
        for peer in file.peers.iter().filter(|p| **p != from) {
            self.announce_in_background(*peer, &file, false);
        }
    }

    fn announce_in_background(self: &Arc<Self>, peer: DeviceId, file: &LinkedFile, resolved: bool) {
        let (id, version, content) = (file.id, file.version.clone(), header(&file.content));
        let msg = if resolved {
            SyncMessage::Resolved {
                id,
                version,
                content,
            }
        } else {
            SyncMessage::Changed {
                id,
                version,
                content,
            }
        };
        let (core, file) = (self.clone(), file.clone());
        tokio::spawn(async move {
            match core.announce(peer, msg, &file).await {
                // Sent with the summary when the device connects again.
                Ok(()) | Err(Error::Offline) => {}
                Err(e) => core.emit(SyncEvent::Error {
                    id: file.id,
                    message: e.to_string(),
                }),
            }
        });
    }

    /// Handles one announcement or notice from `peer` and returns the answer, if any.
    fn handle_message(
        self: &Arc<Self>,
        peer: DeviceId,
        msg: SyncMessage,
        hashes: Vec<[u8; 32]>,
    ) -> Result<Option<SyncMessage>> {
        let content_of = |c: ContentHeader| Content {
            size: c.size,
            chunk_size: c.chunk_size,
            hashes: hashes.clone(),
        };
        match msg {
            SyncMessage::Link {
                id,
                name,
                version,
                content,
            } => {
                if let Some(mut file) = self.get(&id) {
                    if !file.peers.contains(&peer) {
                        file.peers.push(peer);
                        lock(&self.journal).put(file)?;
                    }
                    return self.on_version(peer, id, version, content_of(content));
                }
                let pending = Pending {
                    kind: Incoming::New { name },
                    version,
                    content: content_of(content),
                };
                lock(&self.pending).insert((peer, id), pending);
                Ok(Some(SyncMessage::Reply { id, want: true }))
            }
            SyncMessage::Changed {
                id,
                version,
                content,
            }
            | SyncMessage::Resolved {
                id,
                version,
                content,
            } => {
                self.linked_with(&id, &peer)?;
                self.on_version(peer, id, version, content_of(content))
            }
            SyncMessage::Unlinked { id } => {
                let mut file = self.linked_with(&id, &peer)?;
                file.peers.retain(|p| *p != peer);
                self.save(&file, peer, LogKind::PeerLeft(peer))?;
                self.emit(SyncEvent::PeerUnlinked { id, peer });
                Ok(None)
            }
            SyncMessage::Hashes { .. }
            | SyncMessage::Summary { .. }
            | SyncMessage::Reply { .. } => Err(Error::Refused("unexpected message")),
        }
    }

    /// The record of `id`, if it is linked with `peer`. Anything else is refused.
    fn linked_with(&self, id: &LinkId, peer: &DeviceId) -> Result<LinkedFile> {
        self.get(id)
            .filter(|f| f.peers.contains(peer))
            .ok_or(Error::Refused("not linked with this device"))
    }

    fn on_version(
        self: &Arc<Self>,
        peer: DeviceId,
        id: LinkId,
        version: VersionVector,
        content: Content,
    ) -> Result<Option<SyncMessage>> {
        let mut file = self.get(&id).ok_or(Error::Refused("unknown link"))?;
        let decision = decide(&file, &version, &content.hashes);
        let want = match decision {
            Decision::Ignore => {
                if file.version.compare(&version) == Order::After {
                    // The other device is behind: send it this version.
                    self.announce_in_background(peer, &file, false);
                }
                false
            }
            Decision::AdoptVersion => {
                file.version = file.version.merged(&version);
                if let State::Conflict { .. } = file.state {
                    // Both devices ended up with the same contents.
                    let waiting = self.conflict_path(&file);
                    let _ = std::fs::remove_file(waiting);
                    file.state = State::Current;
                }
                self.save(&file, peer, LogKind::Received)?;
                self.forward(id, peer);
                false
            }
            Decision::Update | Decision::Conflict => {
                let kind = if decision == Decision::Update {
                    Incoming::Update
                } else {
                    Incoming::Conflict
                };
                if decision == Decision::Update && !matches!(file.state, State::Conflict { .. }) {
                    file.state = State::Updating;
                    lock(&self.journal).put(file)?;
                }
                let pending = Pending {
                    kind,
                    version,
                    content,
                };
                lock(&self.pending).insert((peer, id), pending);
                true
            }
        };
        Ok(Some(SyncMessage::Reply { id, want }))
    }

    /// Places a received, verified version. Called from the transfer's finishing step.
    fn commit(&self, peer: DeviceId, id: LinkId, pending: Pending, part: &Path) -> Result<PathBuf> {
        match pending.kind {
            Incoming::New { ref name } => self.commit_new(peer, id, name, &pending, part),
            Incoming::Update => {
                let file = self.get(&id).ok_or(Error::Refused("unknown link"))?;
                if let Some(hook) = &self.cfg.before_commit {
                    hook(&file.path);
                }
                // Changed here since this device last looked: keep the local edit and
                // treat the received version as a conflict.
                let unchanged = stat(&file.path).map(|(seen, _)| seen) == Some(file.seen);
                if unchanged {
                    self.commit_update(peer, file, &pending, part)
                } else {
                    self.commit_conflict(peer, file, &pending, part)
                }
            }
            Incoming::Conflict => {
                let file = self.get(&id).ok_or(Error::Refused("unknown link"))?;
                self.commit_conflict(peer, file, &pending, part)
            }
        }
    }

    fn commit_new(
        &self,
        peer: DeviceId,
        id: LinkId,
        name: &str,
        pending: &Pending,
        part: &Path,
    ) -> Result<PathBuf> {
        let dir = lock(&self.linked_root).join(date_folder(self.now()));
        std::fs::create_dir_all(&dir).map_err(|e| io_err("create directory", &dir, e))?;
        let rel = RelPath::new(vec![name.to_owned()])?;
        let dest = ec_transfer::path::unique_destination(&dir, &rel)?;
        std::fs::rename(part, &dest).map_err(|e| io_err("move into place", part, e))?;
        let (seen, identity) = stat(&dest).ok_or(Error::Refused("saved file disappeared"))?;
        let file = LinkedFile {
            id,
            name: file_name(&dest),
            path: dest.clone(),
            identity,
            peers: vec![peer],
            version: pending.version.clone(),
            content: pending.content.clone(),
            seen,
            state: State::Current,
        };
        self.save(&file, peer, LogKind::Linked)?;
        self.refresh_watch();
        self.emit(SyncEvent::Linked {
            id,
            from: peer,
            path: dest.clone(),
        });
        Ok(dest)
    }

    fn commit_update(
        &self,
        peer: DeviceId,
        mut file: LinkedFile,
        pending: &Pending,
        part: &Path,
    ) -> Result<PathBuf> {
        std::fs::rename(part, &file.path).map_err(|e| io_err("replace", &file.path, e))?;
        let (seen, identity) = stat(&file.path).ok_or(Error::Refused("saved file disappeared"))?;
        self.watcher
            .expect_write(file.id, seen, pending.content.clone());
        if let State::Conflict { version, .. } = &file.state
            && pending.version.compare(version) != Order::Concurrent
        {
            // The new version includes the waiting one: the conflict is over.
            let _ = std::fs::remove_file(self.conflict_path(&file));
            file.state = State::Current;
        }
        if !matches!(file.state, State::Conflict { .. }) {
            file.state = State::Current;
        }
        file.version = file.version.merged(&pending.version);
        file.content = pending.content.clone();
        file.seen = seen;
        file.identity = identity;
        self.save(&file, peer, LogKind::Received)?;
        self.emit(SyncEvent::Updated { id: file.id });
        Ok(file.path)
    }

    fn commit_conflict(
        &self,
        peer: DeviceId,
        mut file: LinkedFile,
        pending: &Pending,
        part: &Path,
    ) -> Result<PathBuf> {
        let waiting = self.conflict_path(&file);
        if let Some(dir) = waiting.parent() {
            std::fs::create_dir_all(dir).map_err(|e| io_err("create directory", dir, e))?;
        }
        // A newer conflicting version replaces an older waiting one.
        std::fs::rename(part, &waiting).map_err(|e| io_err("keep", &waiting, e))?;
        file.state = State::Conflict {
            theirs: pending.content.clone(),
            version: pending.version.clone(),
            from: peer,
            at_ms: self.now(),
        };
        self.save(&file, peer, LogKind::Conflict)?;
        self.emit(SyncEvent::Conflict { id: file.id });
        Ok(waiting)
    }

    fn resolve(self: &Arc<Self>, id: LinkId, keep_theirs: bool) -> Result<()> {
        let mut file = self.get(&id).ok_or(Error::Refused("unknown link"))?;
        let State::Conflict {
            theirs,
            version: their_version,
            ..
        } = file.state.clone()
        else {
            return Err(Error::Refused("no conflict to resolve"));
        };
        let waiting = self.conflict_path(&file);
        let mut version = file.version.merged(&their_version);
        version.bump(self.cfg.me);
        if keep_theirs {
            (self.cfg.trash)(&file.path).map_err(|e| io_err("move to the Trash", &file.path, e))?;
            std::fs::rename(&waiting, &file.path).map_err(|e| io_err("replace", &file.path, e))?;
            file.content = theirs;
        } else {
            (self.cfg.trash)(&waiting).map_err(|e| io_err("move to the Trash", &waiting, e))?;
        }
        let (seen, identity) = stat(&file.path).ok_or(Error::Refused("file disappeared"))?;
        self.watcher.expect_write(id, seen, file.content.clone());
        file.version = version;
        file.seen = seen;
        file.identity = identity;
        file.state = State::Current;
        self.save(&file, self.cfg.me, LogKind::Resolved)?;
        self.emit(SyncEvent::Resolved { id });
        self.announce_to_peers(id, true);
        Ok(())
    }

    fn leave(self: &Arc<Self>, id: LinkId) -> Result<()> {
        let Some(file) = lock(&self.journal).remove(&id)? else {
            return Ok(());
        };
        let _ = std::fs::remove_file(self.conflict_path(&file));
        self.refresh_watch();
        for peer in file.peers {
            let core = self.clone();
            tokio::spawn(async move {
                if let Some(conn) = core.outbox.conn(&peer)
                    && let Ok((mut send, _recv)) = conn.open_bi(SYNC_KIND).await
                {
                    let _ = write_sync(&mut send, &SyncMessage::Unlinked { id }).await;
                    let _ = send.finish();
                }
            });
        }
        self.emit(SyncEvent::LeftLink { id });
        Ok(())
    }

    fn moved(&self, id: LinkId, to: PathBuf, identity: Option<crate::FileIdentity>) {
        let Some(mut file) = self.get(&id) else {
            return;
        };
        let from = std::mem::replace(&mut file.path, to.clone());
        file.name = file_name(&to);
        file.identity = identity;
        if file.state == State::Missing {
            file.state = State::Current;
        }
        if self
            .save(&file, self.cfg.me, LogKind::Moved { from, to })
            .is_ok()
        {
            self.refresh_watch();
            self.emit(SyncEvent::Moved { id });
        }
    }

    async fn on_watch(self: &Arc<Self>, event: WatchEvent) {
        match event {
            WatchEvent::Changed { id, content, seen } => {
                let Some(mut file) = self.get(&id) else {
                    return;
                };
                file.version.bump(self.cfg.me);
                file.content = content;
                file.seen = seen;
                if matches!(file.state, State::Missing | State::Updating) {
                    file.state = State::Current;
                }
                if self.save(&file, self.cfg.me, LogKind::Changed).is_ok() {
                    self.announce_to_peers(id, false);
                }
            }
            WatchEvent::Moved { id, to, identity } => {
                if in_trash(&to) {
                    let _ = self.leave(id);
                } else {
                    self.moved(id, to, identity);
                }
            }
            WatchEvent::Deleted { id } => {
                let _ = self.leave(id);
            }
            WatchEvent::Missing { id } => {
                if let Some(mut file) = self.get(&id) {
                    file.state = State::Missing;
                    let _ = self.save(&file, self.cfg.me, LogKind::Missing);
                    self.emit(SyncEvent::Missing { id });
                }
            }
            WatchEvent::Found { id, at } => {
                if let Some(mut file) = self.get(&id) {
                    file.path = at;
                    file.state = State::Current;
                    let _ = self.save(&file, self.cfg.me, LogKind::Found);
                    self.refresh_watch();
                    self.emit(SyncEvent::Found { id });
                }
            }
        }
    }

    async fn send_summary(self: &Arc<Self>, peer: DeviceId) {
        let Some(conn) = self.outbox.conn(&peer) else {
            return;
        };
        let entries: Vec<(LinkId, VersionVector)> = lock(&self.journal)
            .files()
            .filter(|f| f.peers.contains(&peer))
            .map(|f| (f.id, f.version.clone()))
            .collect();
        let result = async {
            let (mut send, mut recv) = conn.open_bi(SYNC_KIND).await?;
            let batches: Vec<&[(LinkId, VersionVector)]> = if entries.is_empty() {
                vec![&[]]
            } else {
                entries.chunks(SUMMARY_BATCH).collect()
            };
            let count = batches.len();
            for (n, batch) in batches.into_iter().enumerate() {
                let msg = SyncMessage::Summary {
                    entries: batch.to_vec(),
                    last: n + 1 == count,
                };
                write_sync(&mut send, &msg).await?;
            }
            let answer = timeout(STEP_TIMEOUT, read_sync(&mut recv)).await;
            let _ = send.finish();
            Ok::<_, Error>(answer)
        }
        .await;
        let supported = match result {
            Ok(Ok(Ok(SyncMessage::Reply { id, .. }))) if id == SUMMARY_ACK => Some(true),
            // No answer in time: unknown, try again on the next connection.
            Ok(Err(_)) => None,
            // The stream was stopped or closed: an older device.
            _ => Some(false),
        };
        if let Some(supported) = supported {
            lock(&self.support).insert(peer, supported);
            self.emit(SyncEvent::Support { peer, supported });
        }
    }

    async fn serve_summary(
        self: &Arc<Self>,
        peer: DeviceId,
        first: SyncMessage,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<()> {
        let mut theirs: HashMap<LinkId, VersionVector> = HashMap::new();
        let mut msg = first;
        loop {
            let SyncMessage::Summary { entries, last } = msg else {
                return Err(Error::Refused("expected a summary"));
            };
            theirs.extend(entries);
            if last {
                break;
            }
            msg = timeout(STEP_TIMEOUT, read_sync(&mut recv))
                .await
                .map_err(|_| Error::Refused("summary timeout"))??;
        }
        write_sync(
            &mut send,
            &SyncMessage::Reply {
                id: SUMMARY_ACK,
                want: false,
            },
        )
        .await?;
        let _ = send.finish();
        lock(&self.support).insert(peer, true);

        let shared: Vec<LinkedFile> = lock(&self.journal)
            .files()
            .filter(|f| f.peers.contains(&peer))
            .cloned()
            .collect();
        for file in shared {
            match theirs.get(&file.id) {
                // The other device does not have it (it was offline when the file was
                // linked): send the link.
                None => {
                    let core = self.clone();
                    tokio::spawn(async move {
                        let msg = SyncMessage::Link {
                            id: file.id,
                            name: file.name.clone(),
                            version: file.version.clone(),
                            content: header(&file.content),
                        };
                        let _ = core.announce(peer, msg, &file).await;
                    });
                }
                Some(version) => {
                    if matches!(
                        file.version.compare(version),
                        Order::After | Order::Concurrent
                    ) {
                        self.announce_in_background(peer, &file, false);
                    }
                }
            }
        }
        Ok(())
    }

    async fn serve(
        self: Arc<Self>,
        peer: DeviceId,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<()> {
        let first = timeout(STEP_TIMEOUT, read_sync(&mut recv))
            .await
            .map_err(|_| Error::Refused("timeout"))??;
        if let SyncMessage::Summary { .. } = first {
            return self.serve_summary(peer, first, send, recv).await;
        }
        let (msg, hashes) = read_content(&mut recv, first).await?;
        if let Some(reply) = self.handle_message(peer, msg, hashes)? {
            write_sync(&mut send, &reply).await?;
        }
        let _ = send.finish();
        Ok(())
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

async fn watch_loop(core: Weak<Core>, mut events: mpsc::UnboundedReceiver<WatchEvent>) {
    while let Some(event) = events.recv().await {
        let Some(core) = core.upgrade() else {
            return;
        };
        core.on_watch(event).await;
    }
}

impl Service for SyncService {
    fn handle_bi(
        self: Arc<Self>,
        conn: PeerConnection,
        send: SendStream,
        recv: RecvStream,
    ) -> ServiceFuture {
        let core = self.core.clone();
        Box::pin(async move {
            let peer = conn.peer();
            if let Err(e) = core.serve(peer, send, recv).await {
                tracing::info!("linked files message from {} refused: {e}", peer.short());
            }
        })
    }
}

struct LinkedPlacement {
    core: Arc<Core>,
    peer: DeviceId,
    id: LinkId,
    pending: Pending,
}

impl Placer for SyncService {
    fn place(
        &self,
        peer: DeviceId,
        key: [u8; 16],
        entry: &FileEntry,
    ) -> Option<Arc<dyn Placement>> {
        let pending = lock(&self.core.pending).remove(&(peer, key))?;
        let matches = entry.size == pending.content.size
            && entry.chunk_size == pending.content.chunk_size
            && entry.hashes == pending.content.hashes;
        matches.then(|| {
            Arc::new(LinkedPlacement {
                core: self.core.clone(),
                peer,
                id: key,
                pending,
            }) as Arc<dyn Placement>
        })
    }
}

impl Placement for LinkedPlacement {
    fn seed(&self) -> Option<PathBuf> {
        match self.pending.kind {
            Incoming::New { .. } => None,
            Incoming::Update | Incoming::Conflict => self.core.get(&self.id).map(|f| f.path),
        }
    }

    fn commit(&self, part: &Path) -> std::io::Result<PathBuf> {
        let placed = self
            .core
            .commit(self.peer, self.id, self.pending.clone(), part);
        if placed.is_ok() {
            self.core.forward(self.id, self.peer);
        }
        placed.map_err(|e| {
            self.core.emit(SyncEvent::Error {
                id: self.id,
                message: e.to_string(),
            });
            std::io::Error::other(e.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::{Seen, VersionVector};

    const ME: DeviceId = DeviceId::from_bytes([1; 32]);
    const B: DeviceId = DeviceId::from_bytes([2; 32]);
    const C: DeviceId = DeviceId::from_bytes([3; 32]);
    const DAY: u64 = 1_792_411_200_000; // 2026-10-19 12:00 UTC

    struct NoPeers;

    impl Outbox for NoPeers {
        fn send_file(&self, _: DeviceId, _: PathBuf, _: LinkId) -> BoxFuture<Result<()>> {
            Box::pin(async { Err(Error::Offline) })
        }
        fn conn(&self, _: &DeviceId) -> Option<PeerConnection> {
            None
        }
    }

    struct Setup {
        dir: tempfile::TempDir,
        service: Arc<SyncService>,
        trashed: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    fn v(entries: &[(DeviceId, u64)]) -> VersionVector {
        entries.iter().copied().collect()
    }

    fn start() -> Setup {
        let dir = tempfile::tempdir().unwrap();
        let trashed = Arc::new(Mutex::new(Vec::new()));
        let sink = trashed.clone();
        let cfg = SyncConfig {
            me: ME,
            dir: dir.path().join("home/linked"),
            linked_root: dir.path().join("Received/Linked"),
            trash: Arc::new(move |path| {
                sink.lock().unwrap().push(std::fs::read(path)?);
                std::fs::remove_file(path)
            }),
            now_ms: Arc::new(|| DAY),
            watch: WatchConfig {
                quiet: Duration::from_millis(200),
                stat_interval: Duration::from_secs(60),
                roots: vec![dir.path().join("Received/Linked")],
            },
            before_commit: None,
        };
        let (events, _) = broadcast::channel(16);
        let service = SyncService::start(cfg, Arc::new(NoPeers), events).unwrap();
        Setup {
            dir,
            service,
            trashed,
        }
    }

    fn record(path: &Path, peers: Vec<DeviceId>, version: VersionVector) -> LinkedFile {
        let described = describe([9; 16], path).unwrap();
        LinkedFile {
            id: [9; 16],
            name: file_name(path),
            path: path.to_path_buf(),
            identity: described.identity,
            peers,
            version,
            content: described.content,
            seen: described.seen,
            state: State::Current,
        }
    }

    fn content(hash: u8) -> Content {
        Content {
            size: 1,
            chunk_size: 4 * 1024 * 1024,
            hashes: vec![[hash; 32]],
        }
    }

    #[test]
    fn decide_update_ignore_conflict_adopt() {
        let local = LinkedFile {
            id: [9; 16],
            name: "x".into(),
            path: "x".into(),
            identity: None,
            peers: vec![B],
            version: v(&[(ME, 2)]),
            content: content(1),
            seen: Seen::default(),
            state: State::Current,
        };
        let h1 = content(1).hashes;
        let h2 = content(2).hashes;
        assert_eq!(decide(&local, &v(&[(ME, 3)]), &h2), Decision::Update);
        assert_eq!(decide(&local, &v(&[(ME, 1)]), &h2), Decision::Ignore);
        assert_eq!(decide(&local, &v(&[(ME, 2)]), &h1), Decision::Ignore);
        assert_eq!(
            decide(&local, &v(&[(ME, 1), (B, 1)]), &h2),
            Decision::Conflict
        );
        assert_eq!(
            decide(&local, &v(&[(ME, 2), (B, 1)]), &h1),
            Decision::AdoptVersion
        );
    }

    #[tokio::test]
    async fn changed_from_unlinked_peer_is_refused() {
        let s = start();
        let path = s.dir.path().join("Rapor.pdf");
        std::fs::write(&path, b"mine").unwrap();
        let file = record(&path, vec![B], v(&[(ME, 1)]));
        lock(&s.service.core().journal).put(file.clone()).unwrap();

        let msg = SyncMessage::Changed {
            id: file.id,
            version: v(&[(ME, 1), (C, 1)]),
            content: header(&content(7)),
        };
        assert!(
            s.service
                .core()
                .handle_message(C, msg, content(7).hashes)
                .is_err()
        );
        assert_eq!(s.service.files(), vec![file]);
    }

    #[tokio::test]
    async fn first_link_goes_to_dated_folder() {
        let s = start();
        let core = s.service.core().clone();
        for (n, id) in [[4u8; 16], [5u8; 16]].into_iter().enumerate() {
            let msg = SyncMessage::Link {
                id,
                name: "Rapor.pdf".into(),
                version: v(&[(B, 1)]),
                content: header(&content(3)),
            };
            let reply = core.handle_message(B, msg, content(3).hashes).unwrap();
            assert_eq!(reply, Some(SyncMessage::Reply { id, want: true }));

            let entry = FileEntry {
                path: RelPath::new(vec!["Rapor.pdf".into()]).unwrap(),
                size: 1,
                chunk_size: 4 * 1024 * 1024,
                hashes: content(3).hashes,
            };
            let placement = s.service.place(B, id, &entry).unwrap();
            assert_eq!(placement.seed(), None);
            let part = s.dir.path().join(format!("part{n}"));
            std::fs::write(&part, b"x").unwrap();
            let saved = placement.commit(&part).unwrap();
            let expected = if n == 0 { "Rapor.pdf" } else { "Rapor (1).pdf" };
            assert!(
                saved.ends_with(Path::new("Linked/2026-10-19").join(expected)),
                "{}",
                saved.display()
            );
            let file = s.service.files().into_iter().find(|f| f.id == id).unwrap();
            assert_eq!(file.peers, vec![B]);
            assert_eq!(file.path, saved);
        }
    }

    #[tokio::test]
    async fn placement_must_match_the_announced_version() {
        let s = start();
        let core = s.service.core().clone();
        let id = [4u8; 16];
        let msg = SyncMessage::Link {
            id,
            name: "Rapor.pdf".into(),
            version: v(&[(B, 1)]),
            content: header(&content(3)),
        };
        core.handle_message(B, msg, content(3).hashes).unwrap();
        let other = FileEntry {
            path: RelPath::new(vec!["Rapor.pdf".into()]).unwrap(),
            size: 1,
            chunk_size: 4 * 1024 * 1024,
            hashes: content(8).hashes,
        };
        assert!(s.service.place(B, id, &other).is_none());
        // And another device cannot use it either.
        assert!(s.service.place(C, id, &other).is_none());
    }

    #[tokio::test]
    async fn linking_again_adds_a_peer() {
        let s = start();
        let path = s.dir.path().join("Sunum.key");
        std::fs::write(&path, b"slides").unwrap();
        let first = s.service.link(B, path.clone(), false).await.unwrap();
        let second = s.service.link(C, path.clone(), false).await.unwrap();
        assert_eq!(first, second);
        let files = s.service.files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].peers, vec![B, C]);
        assert_eq!(files[0].version, v(&[(ME, 1)]));
    }

    #[tokio::test]
    async fn resolve_keep_theirs_trashes_mine() {
        let s = start();
        let path = s.dir.path().join("Butce.xlsx");
        std::fs::write(&path, b"mine").unwrap();
        let mut file = record(&path, vec![B], v(&[(ME, 2)]));
        let theirs = describe(file.id, &{
            let p = s.dir.path().join("theirs");
            std::fs::write(&p, b"theirs").unwrap();
            p
        })
        .unwrap();
        file.state = State::Conflict {
            theirs: theirs.content.clone(),
            version: v(&[(ME, 1), (B, 1)]),
            from: B,
            at_ms: DAY,
        };
        let core = s.service.core().clone();
        let waiting = core.conflict_path(&file);
        std::fs::create_dir_all(waiting.parent().unwrap()).unwrap();
        std::fs::write(&waiting, b"theirs").unwrap();
        lock(&core.journal).put(file.clone()).unwrap();

        s.service.resolve(file.id, true).unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"theirs");
        assert_eq!(*s.trashed.lock().unwrap(), vec![b"mine".to_vec()]);
        let after = s.service.files().pop().unwrap();
        assert_eq!(after.state, State::Current);
        assert_eq!(after.version, v(&[(ME, 3), (B, 1)]));
        assert_eq!(after.content, theirs.content);
        assert!(!waiting.exists());
    }

    #[test]
    fn dates_are_utc_days() {
        assert_eq!(date_folder(0), "1970-01-01");
        assert_eq!(date_folder(DAY), "2026-10-19");
        assert_eq!(date_folder(951_782_400_000), "2000-02-29");
    }
}
