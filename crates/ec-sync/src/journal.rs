//! The records of linked files and their change log, on disk.
//!
//! Layout under the journal folder:
//!
//! - `journal.bin`: every record (postcard), replaced atomically on each change.
//! - `log/<id hex>.bin`: the change log of one file, at most [`MAX_LOG_ENTRIES`] entries.
//!
//! An unreadable `journal.bin` is moved aside to `journal.broken` and the journal starts
//! empty; the files themselves are untouched and are recovered from the other devices.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ec_core::DeviceId;
use ec_core::fsutil::{create_private_dir, read_optional, write_private};
use serde::{Deserialize, Serialize};

use crate::{Error, Result, VersionVector};

/// Identifies a linked file; the same on every device.
pub type LinkId = [u8; 16];

/// Most log entries kept per file; older ones are dropped.
pub const MAX_LOG_ENTRIES: usize = 200;

const JOURNAL_FILE: &str = "journal.bin";
/// Which devices are known to have each file, kept apart so the journal format of
/// 0.4.0-alpha.1 still reads.
const DELIVERED_FILE: &str = "delivered.bin";
const BROKEN_FILE: &str = "journal.broken";
const LOG_DIR: &str = "log";
const FORMAT: u32 = 1;

/// A new random link id.
pub fn new_link_id() -> LinkId {
    rand::random()
}

/// What a version of the file contains.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Content {
    /// Size in bytes.
    pub size: u64,
    /// Size of every chunk except possibly the last.
    pub chunk_size: u32,
    /// BLAKE3 hash of each chunk.
    pub hashes: Vec<[u8; 32]>,
}

/// What this device last saw on disk: the cheap check before hashing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    /// Size in bytes.
    pub size: u64,
    /// Modification time in nanoseconds since the Unix epoch.
    pub mtime_ns: i128,
}

/// What identifies a file on its volume, across moves and renames.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIdentity {
    /// The device (volume) number.
    pub dev: u64,
    /// The inode number.
    pub ino: u64,
}

/// Where a linked file stands on this device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    /// This device has the latest version it knows of.
    Current,
    /// A newer version is being received.
    Updating,
    /// Another device changed the same version; the user chooses.
    Conflict {
        /// The other version, kept until the user chooses.
        theirs: Content,
        /// Its version vector.
        version: VersionVector,
        /// The device it came from.
        from: DeviceId,
        /// When it arrived, in milliseconds since the Unix epoch.
        at_ms: u64,
        /// Where it waits.
        waiting: PathBuf,
    },
    /// The file cannot be found; the link is kept.
    Missing,
}

/// A linked file as this device knows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedFile {
    /// The link id.
    pub id: LinkId,
    /// The file name on this device.
    pub name: String,
    /// Where the file is on this device.
    pub path: PathBuf,
    /// Its identity on the volume, where the platform has one.
    pub identity: Option<FileIdentity>,
    /// The devices it is linked with.
    pub peers: Vec<DeviceId>,
    /// The version this device has.
    pub version: VersionVector,
    /// What that version contains.
    pub content: Content,
    /// What this device last saw on disk.
    pub seen: Seen,
    /// Where it stands.
    pub state: State,
}

/// What a log entry records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogKind {
    /// The file was linked.
    Linked,
    /// The file was changed on the device that made the entry.
    Changed,
    /// A new version was received.
    Received,
    /// The file was moved or renamed on this device.
    Moved {
        /// Where it was.
        from: PathBuf,
        /// Where it is now.
        to: PathBuf,
    },
    /// Two versions conflicted.
    Conflict,
    /// A conflict was resolved.
    Resolved,
    /// This device left the link.
    Unlinked,
    /// Another device left the link.
    PeerLeft(DeviceId),
    /// The file could not be found.
    Missing,
    /// The file was found again.
    Found,
}

/// One entry of a file's change log. No file contents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    /// When, in milliseconds since the Unix epoch.
    pub at_ms: u64,
    /// The device it happened on.
    pub by: DeviceId,
    /// What happened.
    pub kind: LogKind,
    /// The version afterwards.
    pub version: VersionVector,
    /// The file size afterwards.
    pub size: u64,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: u32,
    files: Vec<LinkedFile>,
}

/// The linked files of this device.
pub struct Journal {
    dir: PathBuf,
    files: BTreeMap<LinkId, LinkedFile>,
    delivered: BTreeMap<LinkId, Vec<DeviceId>>,
    recovered: bool,
}

impl Journal {
    /// Opens the journal in `dir`, creating the folder if needed.
    pub fn open(dir: &Path) -> Result<Self> {
        create_private_dir(dir)?;
        create_private_dir(&dir.join(LOG_DIR))?;
        let path = dir.join(JOURNAL_FILE);
        let mut recovered = false;
        let files = match read_optional(&path)? {
            None => Vec::new(),
            Some(data) => match postcard::from_bytes::<Stored>(&data) {
                Ok(stored) if stored.format == FORMAT => stored.files,
                _ => {
                    let broken = dir.join(BROKEN_FILE);
                    tracing::warn!(
                        "linked files journal is unreadable; kept as {}",
                        broken.display()
                    );
                    std::fs::rename(&path, &broken).map_err(|source| Error::Io {
                        action: "move aside",
                        path: path.clone(),
                        source,
                    })?;
                    recovered = true;
                    Vec::new()
                }
            },
        };
        let delivered = read_optional(&dir.join(DELIVERED_FILE))?
            .and_then(|data| postcard::from_bytes(&data).ok())
            .unwrap_or_default();
        Ok(Self {
            dir: dir.to_path_buf(),
            files: files.into_iter().map(|f| (f.id, f)).collect(),
            delivered,
            recovered,
        })
    }

    fn save(&self) -> Result<()> {
        let stored = Stored {
            format: FORMAT,
            files: self.files.values().cloned().collect(),
        };
        let data = postcard::to_stdvec(&stored).map_err(|_| Error::Encode("journal"))?;
        Ok(write_private(&self.dir.join(JOURNAL_FILE), &data)?)
    }

    fn save_delivered(&self) -> Result<()> {
        let data = postcard::to_stdvec(&self.delivered).map_err(|_| Error::Encode("delivered"))?;
        Ok(write_private(&self.dir.join(DELIVERED_FILE), &data)?)
    }

    /// Records that `peer` has the file `id` (it received or announced it).
    pub fn mark_delivered(&mut self, id: &LinkId, peer: DeviceId) -> Result<()> {
        let peers = self.delivered.entry(*id).or_default();
        if peers.contains(&peer) {
            return Ok(());
        }
        peers.push(peer);
        self.save_delivered()
    }

    /// Whether `peer` is known to have had the file `id`.
    pub fn was_delivered(&self, id: &LinkId, peer: &DeviceId) -> bool {
        self.delivered.get(id).is_some_and(|p| p.contains(peer))
    }

    fn log_path(&self, id: &LinkId) -> PathBuf {
        self.dir
            .join(LOG_DIR)
            .join(format!("{}.bin", hex::encode(id)))
    }

    /// Whether the journal was unreadable and started empty.
    pub fn recovered(&self) -> bool {
        self.recovered
    }

    /// The record of `id`.
    pub fn get(&self, id: &LinkId) -> Option<&LinkedFile> {
        self.files.get(id)
    }

    /// Every record.
    pub fn files(&self) -> impl Iterator<Item = &LinkedFile> {
        self.files.values()
    }

    /// Adds or replaces a record and saves the journal.
    pub fn put(&mut self, file: LinkedFile) -> Result<()> {
        self.files.insert(file.id, file);
        self.save()
    }

    /// Removes a record and its log.
    pub fn remove(&mut self, id: &LinkId) -> Result<Option<LinkedFile>> {
        let removed = self.files.remove(id);
        if removed.is_some() {
            self.save()?;
        }
        if self.delivered.remove(id).is_some() {
            self.save_delivered()?;
        }
        let log = self.log_path(id);
        match std::fs::remove_file(&log) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(Error::Io {
                    action: "remove",
                    path: log,
                    source,
                });
            }
        }
        Ok(removed)
    }

    /// The change log of `id`, oldest first.
    pub fn log(&self, id: &LinkId) -> Result<Vec<LogEntry>> {
        let Some(data) = read_optional(&self.log_path(id))? else {
            return Ok(Vec::new());
        };
        // The log is only informational; an unreadable one starts over.
        Ok(postcard::from_bytes(&data).unwrap_or_default())
    }

    /// Adds an entry to the log of `id`, dropping the oldest beyond [`MAX_LOG_ENTRIES`].
    pub fn append(&mut self, id: &LinkId, entry: LogEntry) -> Result<()> {
        let mut log = self.log(id)?;
        log.push(entry);
        let excess = log.len().saturating_sub(MAX_LOG_ENTRIES);
        log.drain(..excess);
        let data = postcard::to_stdvec(&log).map_err(|_| Error::Encode("log"))?;
        Ok(write_private(&self.log_path(id), &data)?)
    }

    /// Removes `peer` from every file's peers. The files stay.
    pub fn remove_peer(&mut self, peer: &DeviceId) -> Result<()> {
        let mut changed = false;
        for file in self.files.values_mut() {
            let before = file.peers.len();
            file.peers.retain(|p| p != peer);
            changed |= file.peers.len() != before;
        }
        if changed {
            self.save()?;
        }
        for peers in self.delivered.values_mut() {
            peers.retain(|p| p != peer);
        }
        self.save_delivered()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: DeviceId = DeviceId::from_bytes([1; 32]);
    const B: DeviceId = DeviceId::from_bytes([2; 32]);
    const C: DeviceId = DeviceId::from_bytes([3; 32]);

    fn sample(peers: Vec<DeviceId>) -> LinkedFile {
        LinkedFile {
            id: [7; 16],
            name: "Rapor.pdf".into(),
            path: "/tmp/Rapor.pdf".into(),
            identity: Some(FileIdentity { dev: 1, ino: 2 }),
            peers,
            version: [(A, 3), (B, 1)].into_iter().collect(),
            content: Content {
                size: 10,
                chunk_size: 4 * 1024 * 1024,
                hashes: vec![[9; 32]],
            },
            seen: Seen {
                size: 10,
                mtime_ns: 1_700_000_000_000_000_000,
            },
            state: State::Conflict {
                theirs: Content::default(),
                version: [(B, 2)].into_iter().collect(),
                from: B,
                at_ms: 5,
                waiting: "/tmp/Linked/.waiting/x".into(),
            },
        }
    }

    fn entry(n: u64) -> LogEntry {
        LogEntry {
            at_ms: n,
            by: A,
            kind: LogKind::Changed,
            version: [(A, n)].into_iter().collect(),
            size: n,
        }
    }

    #[test]
    fn records_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let file = sample(vec![B]);
        {
            let mut journal = Journal::open(dir.path()).unwrap();
            journal.put(file.clone()).unwrap();
        }
        let journal = Journal::open(dir.path()).unwrap();
        assert_eq!(journal.get(&file.id), Some(&file));
        assert!(!journal.recovered());
    }

    #[test]
    fn log_is_trimmed_to_200() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path()).unwrap();
        let id = [7; 16];
        for n in 0..250 {
            journal.append(&id, entry(n)).unwrap();
        }
        let log = journal.log(&id).unwrap();
        assert_eq!(log.len(), MAX_LOG_ENTRIES);
        assert_eq!(log[0], entry(50));
        assert_eq!(log[199], entry(249));
    }

    #[test]
    fn broken_journal_is_moved_aside() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(JOURNAL_FILE), b"not a journal").unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        assert_eq!(journal.files().count(), 0);
        assert!(journal.recovered());
        assert_eq!(
            std::fs::read(dir.path().join(BROKEN_FILE)).unwrap(),
            b"not a journal"
        );
        assert!(!dir.path().join(JOURNAL_FILE).exists());
    }

    #[test]
    fn removing_a_peer_keeps_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path()).unwrap();
        let file = sample(vec![B, C]);
        journal.put(file.clone()).unwrap();
        journal.remove_peer(&B).unwrap();
        assert_eq!(journal.get(&file.id).unwrap().peers, vec![C]);
        let reopened = Journal::open(dir.path()).unwrap();
        assert_eq!(reopened.get(&file.id).unwrap().peers, vec![C]);
    }

    #[test]
    fn removing_a_file_removes_its_log() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(dir.path()).unwrap();
        let file = sample(vec![B]);
        journal.put(file.clone()).unwrap();
        journal.append(&file.id, entry(1)).unwrap();
        assert_eq!(journal.remove(&file.id).unwrap(), Some(file.clone()));
        assert!(journal.get(&file.id).is_none());
        assert!(journal.log(&file.id).unwrap().is_empty());
    }
}
