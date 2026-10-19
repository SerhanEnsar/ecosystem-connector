//! Watching linked files without wearing the device out.
//!
//! A file is only hashed when its size or modification time changed, and only after
//! [`WatchConfig::quiet`] without further changes. File system events (FSEvents,
//! inotify) say when to look; a cheap periodic check of size and modification time
//! covers platforms and folders where events are unreliable.
//!
//! A file that is no longer at its path is looked for by its identity on the volume
//! (and then by size, modification time and first chunk) under the configured roots:
//! found, it was moved. Not found, it was deleted only if the file system reported a
//! delete; otherwise it is *missing* (another volume, an unplugged disk, lost access)
//! and the link is kept.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ec_transfer::manifest::{chunk_size_for, hash_chunks};
use notify::{EventKind, RecursiveMode, Watcher as _};
use tokio::sync::mpsc;

use crate::identity::stat;
use crate::{Content, Error, FileIdentity, LinkId, Result, Seen};

/// How long a deleted file's remove event is remembered.
const REMOVE_MEMORY: Duration = Duration::from_secs(10);
/// How deep the roots are searched for a moved file.
const SEARCH_DEPTH: usize = 8;

/// How to watch.
#[derive(Clone, Debug)]
pub struct WatchConfig {
    /// How long a file must stay unchanged before it is checked.
    pub quiet: Duration,
    /// How often sizes and modification times are checked without an event.
    pub stat_interval: Duration,
    /// Folders searched for a file that is no longer at its path.
    pub roots: Vec<PathBuf>,
}

/// A file to watch, as the journal last recorded it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watched {
    /// The link.
    pub id: LinkId,
    /// Where the file is.
    pub path: PathBuf,
    /// Its size and modification time when last checked.
    pub seen: Seen,
    /// Its contents when last checked.
    pub content: Content,
    /// Its identity on the volume.
    pub identity: Option<FileIdentity>,
}

/// What the watcher noticed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchEvent {
    /// The contents changed.
    Changed {
        /// The link.
        id: LinkId,
        /// The new contents.
        content: Content,
        /// The new size and modification time.
        seen: Seen,
    },
    /// The file was moved or renamed.
    Moved {
        /// The link.
        id: LinkId,
        /// Where it is now.
        to: PathBuf,
        /// Its identity there.
        identity: Option<FileIdentity>,
    },
    /// The file was deleted. It is no longer watched.
    Deleted {
        /// The link.
        id: LinkId,
    },
    /// The file cannot be found, and it was not deleted as far as the watcher knows.
    Missing {
        /// The link.
        id: LinkId,
    },
    /// A missing file is back.
    Found {
        /// The link.
        id: LinkId,
        /// Where it is.
        at: PathBuf,
    },
}

/// The result of checking one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheckResult {
    /// Size and modification time are as recorded; nothing was read.
    Unchanged,
    /// Only the modification time (or size records) changed; the contents are the same.
    Touched(Seen),
    /// The contents changed.
    Changed(Content, Seen),
    /// The file changed while it was being hashed; check again later.
    Busy,
    /// There is no file at the path.
    Gone,
}

/// Hashes the file at `path` and describes it for watching.
pub fn describe(id: LinkId, path: &Path) -> Result<Watched> {
    let (seen, identity) = stat(path).ok_or_else(|| Error::Io {
        action: "read",
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "not a regular file"),
    })?;
    Ok(Watched {
        id,
        path: path.to_path_buf(),
        seen,
        content: hash_file(path, seen.size)?,
        identity,
    })
}

/// The chunk hashes of the file at `path`, which has `size` bytes.
pub fn hash_file(path: &Path, size: u64) -> Result<Content> {
    let chunk_size = chunk_size_for(size)?;
    Ok(Content {
        size,
        chunk_size,
        hashes: hash_chunks(path, size, chunk_size)?,
    })
}

/// Checks one file, hashing it only if its size or modification time changed.
/// `hashes` counts the files hashed.
pub fn check(file: &Watched, hashes: &AtomicU64) -> CheckResult {
    let Some((seen, _)) = stat(&file.path) else {
        return CheckResult::Gone;
    };
    if seen == file.seen {
        return CheckResult::Unchanged;
    }
    hashes.fetch_add(1, Ordering::Relaxed);
    let Ok(content) = hash_file(&file.path, seen.size) else {
        return CheckResult::Busy;
    };
    // Written to while it was read: the hashes may mix two versions.
    if stat(&file.path).map(|(after, _)| after) != Some(seen) {
        return CheckResult::Busy;
    }
    if content == file.content {
        CheckResult::Touched(seen)
    } else {
        CheckResult::Changed(content, seen)
    }
}

struct Entry {
    file: Watched,
    /// The path as file system events report it (folders resolved, as FSEvents does).
    event_path: PathBuf,
    /// When a change was noticed; checked once `quiet` has passed since.
    dirty_since: Option<Instant>,
    /// When the file was first found gone.
    gone_since: Option<Instant>,
    missing: bool,
}

impl Entry {
    fn new(file: Watched) -> Self {
        Self {
            event_path: event_path(&file.path),
            file,
            dirty_since: None,
            gone_since: None,
            missing: false,
        }
    }
}

struct Inner {
    entries: HashMap<LinkId, Entry>,
    /// Paths the file system reported as removed, and when.
    removed: HashMap<PathBuf, Instant>,
    dirs: HashSet<PathBuf>,
    stat_interval: Duration,
}

/// Watches linked files and reports [`WatchEvent`]s. Stops when dropped.
pub struct Watcher {
    inner: Arc<Mutex<Inner>>,
    fs: Arc<Mutex<notify::RecommendedWatcher>>,
    hashes: Arc<AtomicU64>,
    task: tokio::task::JoinHandle<()>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// `path` with its folder resolved (`/var` → `/private/var` on macOS), the form file
/// system events use. The path itself is not resolved: it may be gone.
fn event_path(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(dir), Some(name)) => std::fs::canonicalize(dir)
            .map(|dir| dir.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

impl Watcher {
    /// Starts watching `files`. Must run inside a tokio runtime.
    pub fn start(
        cfg: WatchConfig,
        files: Vec<Watched>,
        events: mpsc::UnboundedSender<WatchEvent>,
    ) -> Result<Self> {
        let (fs_tx, fs_rx) = mpsc::unbounded_channel();
        let fs = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            if let Ok(event) = event {
                let _ = fs_tx.send(event);
            }
        })
        .map_err(|e| Error::Watch(e.to_string()))?;
        let inner = Arc::new(Mutex::new(Inner {
            entries: files.into_iter().map(|f| (f.id, Entry::new(f))).collect(),
            removed: HashMap::new(),
            dirs: HashSet::new(),
            stat_interval: cfg.stat_interval,
        }));
        let fs = Arc::new(Mutex::new(fs));
        update_dirs(&inner, &fs);
        let hashes = Arc::new(AtomicU64::new(0));
        let task = tokio::spawn(run(
            cfg,
            inner.clone(),
            fs.clone(),
            hashes.clone(),
            fs_rx,
            events,
        ));
        Ok(Self {
            inner,
            fs,
            hashes,
            task,
        })
    }

    /// Watches exactly `files` from now on. Files already watched keep their state.
    pub fn set(&self, files: Vec<Watched>) {
        {
            let mut inner = lock(&self.inner);
            let mut old = std::mem::take(&mut inner.entries);
            for file in files {
                let entry = match old.remove(&file.id) {
                    Some(entry) if entry.file.path == file.path => Entry { file, ..entry },
                    _ => Entry::new(file),
                };
                inner.entries.insert(entry.file.id, entry);
            }
        }
        update_dirs(&self.inner, &self.fs);
    }

    /// Checks sizes and modification times this often from now on.
    pub fn set_stat_interval(&self, interval: Duration) {
        lock(&self.inner).stat_interval = interval;
    }

    /// Records that this device itself just wrote `id` (a received version), so the
    /// write is not reported as a change.
    pub fn expect_write(&self, id: LinkId, seen: Seen, content: Content) {
        if let Some(entry) = lock(&self.inner).entries.get_mut(&id) {
            entry.file.seen = seen;
            entry.file.content = content;
            entry.dirty_since = None;
            entry.gone_since = None;
            entry.missing = false;
        }
    }

    /// Number of files hashed so far.
    pub fn hash_count(&self) -> u64 {
        self.hashes.load(Ordering::Relaxed)
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Watches the folder of every file (not recursively).
fn update_dirs(inner: &Mutex<Inner>, fs: &Mutex<notify::RecommendedWatcher>) {
    let mut inner = lock(inner);
    let wanted: HashSet<PathBuf> = inner
        .entries
        .values()
        .filter_map(|e| e.file.path.parent().map(Path::to_path_buf))
        .filter(|dir| dir.is_dir())
        .collect();
    let mut fs = lock(fs);
    for dir in inner.dirs.difference(&wanted) {
        let _ = fs.unwatch(dir);
    }
    for dir in wanted.difference(&inner.dirs) {
        if let Err(e) = fs.watch(dir, RecursiveMode::NonRecursive) {
            // The periodic check still covers it.
            tracing::debug!("cannot watch {}: {e}", dir.display());
        }
    }
    inner.dirs = wanted;
}

async fn run(
    cfg: WatchConfig,
    inner: Arc<Mutex<Inner>>,
    fs: Arc<Mutex<notify::RecommendedWatcher>>,
    hashes: Arc<AtomicU64>,
    mut fs_events: mpsc::UnboundedReceiver<notify::Event>,
    events: mpsc::UnboundedSender<WatchEvent>,
) {
    let mut tick = tokio::time::interval((cfg.quiet / 2).max(Duration::from_millis(20)));
    let mut last_poll = Instant::now();
    loop {
        tokio::select! {
            Some(event) = fs_events.recv() => on_fs_event(&inner, &event),
            _ = tick.tick() => {
                let interval = lock(&inner).stat_interval;
                if last_poll.elapsed() >= interval {
                    last_poll = Instant::now();
                    poll(&cfg, &inner, &fs, &events).await;
                }
                process_due(&cfg, &inner, &fs, &hashes, &events).await;
            }
        }
    }
}

fn on_fs_event(inner: &Mutex<Inner>, event: &notify::Event) {
    let now = Instant::now();
    let mut inner = lock(inner);
    for path in &event.paths {
        if matches!(event.kind, EventKind::Remove(_)) {
            inner.removed.insert(path.clone(), now);
        }
        for entry in inner.entries.values_mut() {
            if &entry.event_path == path || &entry.file.path == path {
                entry.dirty_since = Some(now);
            }
        }
    }
    inner.removed.retain(|_, at| at.elapsed() < REMOVE_MEMORY);
}

/// The cheap check: sizes and modification times only.
async fn poll(
    cfg: &WatchConfig,
    inner: &Arc<Mutex<Inner>>,
    fs: &Mutex<notify::RecommendedWatcher>,
    events: &mpsc::UnboundedSender<WatchEvent>,
) {
    let files: Vec<(Watched, bool)> = lock(inner)
        .entries
        .values()
        .map(|e| (e.file.clone(), e.missing))
        .collect();
    let roots = cfg.roots.clone();
    let found = tokio::task::spawn_blocking(move || {
        files
            .into_iter()
            .map(|(file, missing)| {
                let now = stat(&file.path);
                let found = if missing && now.is_none() {
                    find_moved(&file, &roots)
                } else {
                    None
                };
                (file.id, now, found)
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    let mut moved = false;
    {
        let now = Instant::now();
        let mut inner = lock(inner);
        for (id, on_disk, found) in found {
            let Some(entry) = inner.entries.get_mut(&id) else {
                continue;
            };
            if entry.missing {
                let at = match (on_disk, found) {
                    (Some(_), _) => Some(entry.file.path.clone()),
                    (None, Some((at, identity))) => {
                        entry.file.identity = identity;
                        Some(at)
                    }
                    (None, None) => None,
                };
                if let Some(at) = at {
                    moved |= at != entry.file.path;
                    entry.file.path = at.clone();
                    entry.event_path = event_path(&at);
                    entry.missing = false;
                    entry.gone_since = None;
                    entry.dirty_since = Some(now);
                    let _ = events.send(WatchEvent::Found { id, at });
                }
                continue;
            }
            let changed = on_disk.is_none_or(|(seen, _)| seen != entry.file.seen);
            if changed && entry.dirty_since.is_none() {
                entry.dirty_since = Some(now);
            }
        }
    }
    if moved {
        update_dirs(inner, fs);
    }
}

/// Checks the files that have been quiet long enough since a change was noticed.
async fn process_due(
    cfg: &WatchConfig,
    inner: &Arc<Mutex<Inner>>,
    fs: &Mutex<notify::RecommendedWatcher>,
    hashes: &Arc<AtomicU64>,
    events: &mpsc::UnboundedSender<WatchEvent>,
) {
    let due: Vec<Watched> = {
        let mut inner = lock(inner);
        inner
            .entries
            .values_mut()
            .filter(|e| !e.missing && e.dirty_since.is_some_and(|t| t.elapsed() >= cfg.quiet))
            .map(|e| {
                e.dirty_since = None;
                e.file.clone()
            })
            .collect()
    };
    for file in due {
        let id = file.id;
        let (checked, hashes) = (file.clone(), hashes.clone());
        let result = tokio::task::spawn_blocking(move || check(&checked, &hashes))
            .await
            .unwrap_or(CheckResult::Busy);
        match result {
            CheckResult::Unchanged => {
                if let Some(entry) = lock(inner).entries.get_mut(&id) {
                    entry.gone_since = None;
                }
            }
            CheckResult::Touched(seen) => {
                if let Some(entry) = lock(inner).entries.get_mut(&id) {
                    entry.file.seen = seen;
                    entry.gone_since = None;
                }
            }
            CheckResult::Changed(content, seen) => {
                let mut inner = lock(inner);
                // Skip if this device wrote it meanwhile (expect_write).
                let Some(entry) = inner.entries.get_mut(&id) else {
                    continue;
                };
                if entry.file.seen == seen {
                    continue;
                }
                entry.file.seen = seen;
                entry.file.content = content.clone();
                entry.gone_since = None;
                let _ = events.send(WatchEvent::Changed { id, content, seen });
            }
            CheckResult::Busy => {
                if let Some(entry) = lock(inner).entries.get_mut(&id) {
                    entry.dirty_since = Some(Instant::now());
                }
            }
            CheckResult::Gone => gone(cfg, inner, fs, events, file).await,
        }
    }
}

/// A file is not at its path: moved, deleted or missing.
async fn gone(
    cfg: &WatchConfig,
    inner: &Arc<Mutex<Inner>>,
    fs: &Mutex<notify::RecommendedWatcher>,
    events: &mpsc::UnboundedSender<WatchEvent>,
    file: Watched,
) {
    let id = file.id;
    let roots = cfg.roots.clone();
    let searched = file.clone();
    let found = tokio::task::spawn_blocking(move || find_moved(&searched, &roots))
        .await
        .ok()
        .flatten();
    if let Some((to, identity)) = found {
        {
            let mut inner = lock(inner);
            let Some(entry) = inner.entries.get_mut(&id) else {
                return;
            };
            entry.file.path = to.clone();
            entry.file.identity = identity;
            entry.event_path = event_path(&to);
            entry.gone_since = None;
            // It may have been edited on the way.
            entry.dirty_since = Some(Instant::now());
        }
        update_dirs(inner, fs);
        let _ = events.send(WatchEvent::Moved { id, to, identity });
        return;
    }

    // Events can arrive late, so give a delete event some time before calling the
    // file missing.
    let grace = (cfg.quiet * 4).max(Duration::from_secs(1));
    let deleted = {
        let mut inner = lock(inner);
        let removed = {
            let event_path = inner.entries.get(&id).map(|e| e.event_path.clone());
            event_path.is_some_and(|p| inner.removed.contains_key(&p))
                || inner.removed.contains_key(&file.path)
        };
        let Some(entry) = inner.entries.get_mut(&id) else {
            return;
        };
        if removed {
            inner.entries.remove(&id);
            true
        } else {
            let since = *entry.gone_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= grace {
                entry.missing = true;
                let _ = events.send(WatchEvent::Missing { id });
            } else {
                entry.dirty_since = Some(Instant::now());
            }
            false
        }
    };
    if deleted {
        update_dirs(inner, fs);
        let _ = events.send(WatchEvent::Deleted { id });
    }
}

/// Looks for `file` under `roots` and its own folder: first by identity, then by size,
/// modification time and first chunk. Links are never followed.
fn find_moved(file: &Watched, roots: &[PathBuf]) -> Option<(PathBuf, Option<FileIdentity>)> {
    let mut places: Vec<PathBuf> = roots.to_vec();
    if let Some(dir) = file.path.parent() {
        places.push(dir.to_path_buf());
    }
    let mut candidates = Vec::new();
    for place in &places {
        collect_files(place, SEARCH_DEPTH, &mut candidates);
    }
    candidates.retain(|(path, _)| path != &file.path);

    if let Some(identity) = file.identity
        && let Some((path, _)) = candidates
            .iter()
            .find(|(_, meta)| FileIdentity::of(meta) == Some(identity))
    {
        return Some((path.clone(), Some(identity)));
    }
    let first = file.content.hashes.first()?;
    candidates
        .iter()
        .filter(|(_, meta)| Seen::of(meta) == file.seen)
        .find(|(path, _)| {
            hash_chunks(path, file.seen.size, file.content.chunk_size)
                .ok()
                .and_then(|h| h.first().copied())
                .as_ref()
                == Some(first)
        })
        .map(|(path, meta)| (path.clone(), FileIdentity::of(meta)))
}

fn collect_files(dir: &Path, depth: usize, out: &mut Vec<(PathBuf, std::fs::Metadata)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_file() {
            out.push((path, meta));
        } else if meta.is_dir() && depth > 0 {
            collect_files(&path, depth - 1, out);
        }
    }
}
