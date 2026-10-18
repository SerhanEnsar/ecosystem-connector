//! Receiving files: the service that answers transfer streams.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ec_core::DeviceId;
use ec_net::{PeerConnection, RecvStream, SendStream, Service, ServiceFuture, read_msg, write_msg};
use tokio::sync::{Notify, Semaphore, mpsc};
use tokio::time::timeout;

use crate::fileio::write_all_at;
use crate::manifest::{FileEntry, MAX_FILES, hash_chunks};
use crate::path::{ensure_real_dir, unique_destination};
use crate::proto::{
    ChunkHeader, Control, MAX_HASHES_PER_MSG, MAX_RANGES_PER_MSG, OpenHint, TransferId, to_ranges,
};
use crate::{Error, Result};

/// Hidden folder inside the receive folder that holds unfinished files.
pub const PARTIAL_DIR: &str = ".ecosystem-connector-partial";
/// Most transfers received at the same time.
const MAX_ACTIVE: usize = 8;
/// Most chunk bytes held in memory at once, across all transfers.
const MEMORY_BUDGET_MIB: u32 = 256;
const STEP_TIMEOUT: Duration = Duration::from_secs(30);
const CHUNK_TIMEOUT: Duration = Duration::from_secs(120);

/// A transfer offer, shown to the user or a policy before accepting.
#[derive(Clone, Debug)]
pub struct Offer {
    /// The sending device.
    pub peer: DeviceId,
    /// Number of files.
    pub files: usize,
    /// Total size in bytes.
    pub total_bytes: u64,
    /// Path of the first file, for display.
    pub first_path: String,
}

/// What happens while receiving.
#[derive(Clone, Debug)]
pub enum ReceiveEvent {
    /// A transfer was accepted. `already_have` bytes were found from an earlier attempt.
    Started {
        /// The sending device.
        peer: DeviceId,
        /// Number of files.
        files: usize,
        /// Total size in bytes.
        total_bytes: u64,
        /// Bytes already present from an interrupted attempt.
        already_have: u64,
    },
    /// Bytes received and verified so far, including `already_have`.
    Progress {
        /// Received bytes.
        received: u64,
        /// Total bytes.
        total: u64,
    },
    /// A file was saved.
    Saved {
        /// Where.
        path: PathBuf,
        /// The sender asked to open it (a fling), at this page if given.
        open: Option<OpenHint>,
    },
    /// The transfer finished.
    Finished,
    /// The sender went away. The partial files stay for a later resume.
    Interrupted {
        /// Why.
        reason: String,
    },
}

/// Decides whether to accept an offer.
pub type AcceptPolicy = Arc<dyn Fn(&Offer) -> bool + Send + Sync>;
/// Receives [`ReceiveEvent`]s.
pub type ReceiveEvents = Arc<dyn Fn(ReceiveEvent) + Send + Sync>;

/// Answers transfer streams and saves files into a folder.
///
/// Register it with an [`ec_net::Router`] for both [`crate::CONTROL_KIND`] and
/// [`crate::CHUNK_KIND`].
pub struct Receiver {
    dir: PathBuf,
    accept: AcceptPolicy,
    events: ReceiveEvents,
    active: Mutex<HashMap<TransferId, Arc<Incoming>>>,
    memory: Arc<Semaphore>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChunkState {
    Needed,
    Writing,
    Done,
}

struct IncomingFile {
    entry: FileEntry,
    part: PathBuf,
    file: Mutex<Option<Arc<File>>>,
    state: Mutex<Vec<ChunkState>>,
}

struct Incoming {
    peer: DeviceId,
    files: Vec<IncomingFile>,
    remaining: AtomicU64,
    received: AtomicU64,
    total: u64,
    acks: mpsc::UnboundedSender<Control>,
    all_done: Notify,
}

impl Receiver {
    /// Saves accepted transfers into `dir`.
    pub fn new(dir: PathBuf, accept: AcceptPolicy, events: ReceiveEvents) -> Self {
        Self {
            dir,
            accept,
            events,
            active: Mutex::new(HashMap::new()),
            memory: Arc::new(Semaphore::new(MEMORY_BUDGET_MIB as usize)),
        }
    }

    fn active(&self) -> std::sync::MutexGuard<'_, HashMap<TransferId, Arc<Incoming>>> {
        // A poisoned lock only means another task panicked; the map itself is still valid.
        self.active.lock().unwrap_or_else(|e| e.into_inner())
    }

    async fn control(
        self: Arc<Self>,
        conn: PeerConnection,
        mut send: SendStream,
        mut recv: RecvStream,
    ) -> Result<()> {
        let peer = conn.peer();
        let (id, entries, open) = read_offer(&mut recv).await?;
        let total: u64 = entries.iter().map(|e| e.size).sum();
        let offer = Offer {
            peer,
            files: entries.len(),
            total_bytes: total,
            first_path: entries
                .first()
                .map(|e| e.path.to_string())
                .unwrap_or_default(),
        };

        let busy = {
            let active = self.active();
            active.len() >= MAX_ACTIVE && !active.contains_key(&id)
        };
        let decline = if busy {
            Some("too many transfers at once")
        } else if !(self.accept)(&offer) {
            Some("declined")
        } else {
            None
        };
        if let Some(reason) = decline {
            write_msg(
                &mut send,
                &Control::Rejected {
                    reason: reason.into(),
                },
            )
            .await?;
            let _ = send.finish();
            let _ = timeout(Duration::from_secs(2), send.stopped()).await;
            return Ok(());
        }

        let dir = self.dir.clone();
        let files = tokio::task::spawn_blocking(move || open_partials(&dir, peer, entries))
            .await
            .map_err(|_| Error::Protocol("prepare task failed"))??;

        let (acks, mut ack_rx) = mpsc::unbounded_channel();
        let remaining: u64 = files.iter().map(|f| f.needed_count()).sum();
        let already: u64 = files.iter().map(|f| f.present_bytes()).sum();
        let incoming = Arc::new(Incoming {
            peer,
            files,
            remaining: AtomicU64::new(remaining),
            received: AtomicU64::new(already),
            total,
            acks,
            all_done: Notify::new(),
        });
        // A retry of the same transfer replaces the previous attempt.
        self.active().insert(id, incoming.clone());
        (self.events)(ReceiveEvent::Started {
            peer,
            files: offer.files,
            total_bytes: total,
            already_have: already,
        });

        let result = async {
            for (index, file) in incoming.files.iter().enumerate() {
                let needed = file.needed_indices();
                for ranges in to_ranges(needed).chunks(MAX_RANGES_PER_MSG) {
                    let msg = Control::Need {
                        file: index as u32,
                        ranges: ranges.to_vec(),
                    };
                    write_msg(&mut send, &msg).await?;
                }
            }
            write_msg(&mut send, &Control::NeedEnd).await?;

            while incoming.remaining.load(Ordering::Acquire) > 0 {
                tokio::select! {
                    Some(ack) = ack_rx.recv() => write_msg(&mut send, &ack).await?,
                    () = incoming.all_done.notified() => {}
                    () = conn.closed() => return Err(Error::Interrupted("sender disconnected".into())),
                }
            }
            while let Ok(ack) = ack_rx.try_recv() {
                write_msg(&mut send, &ack).await?;
            }

            let saved = finalize(&self.dir, &incoming).await?;
            for (index, path) in saved.into_iter().enumerate() {
                write_msg(&mut send, &Control::FileDone { file: index as u32 }).await?;
                (self.events)(ReceiveEvent::Saved { path, open });
            }
            write_msg(&mut send, &Control::Done).await?;
            let _ = send.finish();
            let _ = timeout(Duration::from_secs(5), send.stopped()).await;
            Ok(())
        }
        .await;

        self.active().remove(&id);
        match &result {
            Ok(()) => (self.events)(ReceiveEvent::Finished),
            Err(e) => (self.events)(ReceiveEvent::Interrupted {
                reason: e.to_string(),
            }),
        }
        result
    }

    async fn chunk(self: Arc<Self>, conn: PeerConnection, mut recv: RecvStream) -> Result<()> {
        let mut raw = [0u8; ChunkHeader::LEN];
        timeout(STEP_TIMEOUT, recv.read_exact(&mut raw))
            .await
            .map_err(|_| Error::Protocol("chunk header timeout"))?
            .map_err(|_| Error::Protocol("short chunk header"))?;
        let header = ChunkHeader::decode(&raw);

        let incoming = self.active().get(&header.id).cloned();
        let Some(incoming) = incoming.filter(|i| i.peer == conn.peer()) else {
            let _ = recv.stop(0u32.into());
            return Ok(());
        };
        let file = incoming
            .files
            .get(header.file as usize)
            .ok_or(Error::Protocol("file index out of range"))?;
        let index = header.chunk;
        if index >= file.entry.chunk_count() {
            return Err(Error::Protocol("chunk index out of range"));
        }

        // Claim the chunk, so two copies of it are never written at the same time.
        {
            let mut state = file.lock_state();
            match state[index as usize] {
                ChunkState::Needed => state[index as usize] = ChunkState::Writing,
                ChunkState::Done => {
                    let _ = recv.stop(0u32.into());
                    let _ = incoming.acks.send(Control::Ack {
                        file: header.file,
                        chunk: index,
                        ok: true,
                    });
                    return Ok(());
                }
                ChunkState::Writing => {
                    let _ = recv.stop(0u32.into());
                    return Ok(());
                }
            }
        }

        let outcome = self.receive_chunk(file, index, &mut recv).await;
        let ok = outcome.is_ok();
        {
            let mut state = file.lock_state();
            state[index as usize] = if ok {
                ChunkState::Done
            } else {
                ChunkState::Needed
            };
        }
        let _ = incoming.acks.send(Control::Ack {
            file: header.file,
            chunk: index,
            ok,
        });
        if ok {
            let len = u64::from(file.entry.chunk_len(index));
            let received = incoming.received.fetch_add(len, Ordering::AcqRel) + len;
            (self.events)(ReceiveEvent::Progress {
                received,
                total: incoming.total,
            });
            if incoming.remaining.fetch_sub(1, Ordering::AcqRel) == 1 {
                incoming.all_done.notify_one();
            }
        }
        outcome
    }

    async fn receive_chunk(
        &self,
        file: &IncomingFile,
        index: u32,
        recv: &mut RecvStream,
    ) -> Result<()> {
        let len = file.entry.chunk_len(index) as usize;
        let mib = (len.div_ceil(1024 * 1024) as u32).clamp(1, MEMORY_BUDGET_MIB);
        let _permit = self
            .memory
            .acquire_many(mib)
            .await
            .map_err(|_| Error::Protocol("receiver shutting down"))?;

        let mut data = vec![0u8; len];
        timeout(CHUNK_TIMEOUT, recv.read_exact(&mut data))
            .await
            .map_err(|_| Error::Protocol("chunk timeout"))?
            .map_err(|_| Error::Protocol("short chunk"))?;
        let mut extra = [0u8; 1];
        if !matches!(recv.read(&mut extra).await, Ok(None)) {
            return Err(Error::Protocol("chunk longer than announced"));
        }
        if blake3::hash(&data) != blake3::Hash::from_bytes(file.entry.hashes[index as usize]) {
            return Err(Error::Protocol("chunk hash mismatch"));
        }

        let handle = file
            .file
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or(Error::Protocol("file already closed"))?;
        let offset = file.entry.chunk_offset(index);
        let part = file.part.clone();
        tokio::task::spawn_blocking(move || {
            write_all_at(&handle, &data, offset).map_err(|e| Error::io("write", &part, e))
        })
        .await
        .map_err(|_| Error::Protocol("write task failed"))?
    }
}

impl Service for Receiver {
    fn handle_bi(
        self: Arc<Self>,
        conn: PeerConnection,
        send: SendStream,
        recv: RecvStream,
    ) -> ServiceFuture {
        Box::pin(async move {
            if let Err(e) = self.control(conn, send, recv).await {
                tracing::info!("transfer ended: {e}");
            }
        })
    }

    fn handle_uni(self: Arc<Self>, conn: PeerConnection, recv: RecvStream) -> ServiceFuture {
        Box::pin(async move {
            if let Err(e) = self.chunk(conn, recv).await {
                tracing::debug!("chunk refused: {e}");
            }
        })
    }
}

impl IncomingFile {
    fn lock_state(&self) -> std::sync::MutexGuard<'_, Vec<ChunkState>> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn needed_indices(&self) -> Vec<u32> {
        let state = self.lock_state();
        (0..state.len() as u32)
            .filter(|i| state[*i as usize] != ChunkState::Done)
            .collect()
    }

    fn needed_count(&self) -> u64 {
        self.lock_state()
            .iter()
            .filter(|s| **s != ChunkState::Done)
            .count() as u64
    }

    fn present_bytes(&self) -> u64 {
        let state = self.lock_state();
        (0..state.len() as u32)
            .filter(|i| state[*i as usize] == ChunkState::Done)
            .map(|i| u64::from(self.entry.chunk_len(i)))
            .sum()
    }
}

/// Reads and validates a complete offer from the control stream.
async fn read_offer(
    recv: &mut RecvStream,
) -> Result<(TransferId, Vec<FileEntry>, Option<OpenHint>)> {
    let mut hint = None;
    let Control::Offer {
        id,
        files,
        total_bytes,
    } = timeout(STEP_TIMEOUT, read_msg(recv))
        .await
        .map_err(|_| Error::Protocol("offer timeout"))??
    else {
        return Err(Error::Protocol("expected an offer"));
    };
    if files as usize > MAX_FILES {
        return Err(Error::TooLarge("too many files"));
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    loop {
        let msg = timeout(STEP_TIMEOUT, read_msg(recv))
            .await
            .map_err(|_| Error::Protocol("offer timeout"))??;
        match msg {
            Control::File {
                index,
                path,
                size,
                chunk_size,
            } => {
                if index as usize != entries.len() || entries.len() >= files as usize {
                    return Err(Error::Protocol("file out of order"));
                }
                if entries
                    .last()
                    .is_some_and(|e| e.hashes.len() as u64 != expected_chunks(e))
                {
                    return Err(Error::Protocol("missing chunk hashes"));
                }
                let entry = FileEntry {
                    path,
                    size,
                    chunk_size,
                    hashes: Vec::new(),
                };
                check_file_header(&entry)?;
                entries.push(entry);
            }
            Control::Hashes {
                file,
                start,
                hashes,
            } => {
                let count = entries.len();
                let entry = entries
                    .last_mut()
                    .ok_or(Error::Protocol("hashes before file"))?;
                if file as usize + 1 != count
                    || start as usize != entry.hashes.len()
                    || hashes.len() > MAX_HASHES_PER_MSG
                    || (entry.hashes.len() + hashes.len()) as u64 > expected_chunks(entry)
                {
                    return Err(Error::Protocol("unexpected chunk hashes"));
                }
                entry.hashes.extend(hashes);
            }
            Control::Hint { open, page } => {
                if hint.is_some() {
                    return Err(Error::Protocol("duplicate hint"));
                }
                hint = Some(open.then_some(OpenHint { page }));
            }
            Control::OfferEnd => break,
            _ => return Err(Error::Protocol("unexpected message in offer")),
        }
    }

    if entries.len() != files as usize {
        return Err(Error::Protocol("incomplete offer"));
    }
    for entry in &entries {
        entry.validate()?;
    }
    if entries.iter().map(|e| e.size).sum::<u64>() != total_bytes {
        return Err(Error::Protocol("total size does not match"));
    }
    let mut seen = std::collections::HashSet::new();
    if !entries.iter().all(|e| seen.insert(&e.path)) {
        return Err(Error::Protocol("duplicate path"));
    }
    Ok((id, entries, hint.flatten()))
}

fn expected_chunks(entry: &FileEntry) -> u64 {
    FileEntry::chunk_count_for(entry.size, entry.chunk_size)
}

/// Checks a file's size and chunk size before its hashes arrive, so the hash count
/// is bounded while they are read.
fn check_file_header(entry: &FileEntry) -> Result<()> {
    if !(64 * 1024..=crate::manifest::MAX_CHUNK_SIZE).contains(&entry.chunk_size) {
        return Err(Error::Protocol("chunk size out of range"));
    }
    if expected_chunks(entry) > crate::manifest::MAX_CHUNKS_PER_FILE {
        return Err(Error::TooLarge("file has too many chunks"));
    }
    Ok(())
}

/// Opens (or creates) the partial file of every entry and checks which chunks it
/// already holds, by hashing them. This is what makes interrupted transfers resume.
fn open_partials(dir: &Path, peer: DeviceId, entries: Vec<FileEntry>) -> Result<Vec<IncomingFile>> {
    std::fs::create_dir_all(dir).map_err(|e| Error::io("create directory", dir, e))?;
    let partial_dir = dir.join(PARTIAL_DIR);
    ensure_real_dir(&partial_dir)?;

    entries
        .into_iter()
        .map(|entry| {
            let part = partial_dir.join(format!("{}.part", partial_key(peer, &entry)));
            let existing = match std::fs::symlink_metadata(&part) {
                Ok(meta) if meta.is_file() => meta.len() == entry.size,
                Ok(_) => return Err(Error::UnsafePath("partial file is not a regular file")),
                Err(_) => false,
            };
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&part)
                .map_err(|e| Error::io("open", &part, e))?;

            let state = if existing && entry.size > 0 {
                let present = hash_chunks(&part, entry.size, entry.chunk_size)?;
                present
                    .iter()
                    .zip(&entry.hashes)
                    .map(|(have, want)| {
                        if have == want {
                            ChunkState::Done
                        } else {
                            ChunkState::Needed
                        }
                    })
                    .collect()
            } else {
                file.set_len(entry.size)
                    .map_err(|e| Error::io("resize", &part, e))?;
                vec![ChunkState::Needed; entry.hashes.len()]
            };
            Ok(IncomingFile {
                part,
                file: Mutex::new(Some(Arc::new(file))),
                state: Mutex::new(state),
                entry,
            })
        })
        .collect()
}

/// Names the partial file so that re-sending the same file from the same device finds it.
fn partial_key(peer: DeviceId, entry: &FileEntry) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"ecosystem-connector partial v1");
    hasher.update(peer.as_bytes());
    hasher.update(entry.path.to_string().as_bytes());
    hasher.update(&entry.size.to_le_bytes());
    hasher.update(&entry.chunk_size.to_le_bytes());
    for hash in &entry.hashes {
        hasher.update(hash);
    }
    hex::encode(&hasher.finalize().as_bytes()[..16])
}

/// Flushes every file to disk and moves it to its final, non-colliding name.
async fn finalize(dir: &Path, incoming: &Incoming) -> Result<Vec<PathBuf>> {
    let jobs: Vec<(Option<Arc<File>>, PathBuf, crate::path::RelPath)> = incoming
        .files
        .iter()
        .map(|f| {
            let handle = f.file.lock().unwrap_or_else(|e| e.into_inner()).take();
            (handle, f.part.clone(), f.entry.path.clone())
        })
        .collect();
    let dir = dir.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let saved = jobs
            .into_iter()
            .map(|(handle, part, rel)| {
                if let Some(handle) = handle {
                    handle.sync_all().map_err(|e| Error::io("sync", &part, e))?;
                    // Close before renaming; Windows cannot rename open files.
                    drop(handle);
                }
                let dest = unique_destination(&dir, &rel)?;
                std::fs::rename(&part, &dest).map_err(|e| Error::io("rename", &part, e))?;
                Ok(dest)
            })
            .collect::<Result<Vec<_>>>();
        // Remove the partial folder once nothing is left in it, so the receive folder
        // stays tidy. It fails harmlessly while another transfer still uses it.
        let _ = std::fs::remove_dir(dir.join(PARTIAL_DIR));
        saved
    })
    .await
    .map_err(|_| Error::Protocol("finalize task failed"))?
}
