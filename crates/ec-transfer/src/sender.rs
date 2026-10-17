//! Sending files over one or more network paths.
//!
//! Every needed chunk goes into one shared queue. Each path (a QUIC connection over a
//! particular network, such as Wi-Fi or a USB cable) runs a few workers that take chunks
//! from the queue and wait for the receiver's acknowledgement. A faster path finishes
//! its chunks sooner and so takes more of them; no explicit balancing is needed. When a
//! path fails, its unacknowledged chunks go back into the queue.
//!
//! If the control connection itself fails, the whole attempt is retried: the receiver
//! hashes what it already has and only asks for the rest.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::future::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use ec_core::DeviceId;
use ec_net::{Node, PeerConnection, RecvStream, read_msg, write_msg};
use tokio::sync::{Notify, oneshot};
use tokio::time::timeout;

use crate::fileio::read_exact_at;
use crate::manifest::SourceFile;
use crate::proto::{
    CHUNK_KIND, CONTROL_KIND, ChunkHeader, Control, MAX_HASHES_PER_MSG, TransferId,
};
use crate::{Error, Result};

const ACK_TIMEOUT: Duration = Duration::from_secs(120);
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
const PATH_CHECK_INTERVAL: Duration = Duration::from_secs(2);

/// Returns the peer's current addresses, for example by asking mDNS.
pub type AddrSource =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Vec<SocketAddr>> + Send>> + Send + Sync>;
/// Receives [`SendEvent`]s.
pub type SendEvents = Arc<dyn Fn(SendEvent) + Send + Sync>;

/// How to send.
#[derive(Clone)]
pub struct SendOptions {
    /// Chunks in flight per path. More hides more latency; 8 is plenty on a LAN.
    pub streams_per_path: usize,
    /// How many times to start over after a broken connection.
    pub max_attempts: u32,
    /// Finds the peer's addresses again, for retries and to discover new paths (for
    /// example when a USB cable is plugged in). Without it only the initial addresses
    /// are used.
    pub rediscover: Option<AddrSource>,
}

impl Default for SendOptions {
    fn default() -> Self {
        Self {
            streams_per_path: 8,
            max_attempts: 5,
            rediscover: None,
        }
    }
}

/// What happens while sending.
#[derive(Clone, Debug)]
pub enum SendEvent {
    /// The receiver accepted; `to_send` bytes are missing on its side.
    Accepted {
        /// Bytes that will be sent in this attempt.
        to_send: u64,
        /// Total size of all files.
        total: u64,
    },
    /// A network path to the receiver is in use.
    PathAdded {
        /// The receiver's address on that path.
        addr: SocketAddr,
    },
    /// A path stopped working.
    PathLost {
        /// The receiver's address on that path.
        addr: SocketAddr,
    },
    /// Bytes acknowledged by the receiver in this attempt.
    Progress {
        /// Acknowledged bytes.
        sent: u64,
        /// Bytes to send in this attempt.
        to_send: u64,
        /// Paths currently in use.
        paths: usize,
    },
    /// The attempt failed and will be retried.
    Retrying {
        /// The next attempt number.
        attempt: u32,
        /// Why the previous one failed.
        reason: String,
    },
}

/// Summary of a finished transfer.
#[derive(Clone, Debug)]
pub struct SendReport {
    /// Number of files.
    pub files: usize,
    /// Total size of all files.
    pub total_bytes: u64,
    /// Bytes actually sent in the final attempt (less than the total after a resume).
    pub bytes_sent: u64,
    /// Time from the receiver's acceptance to the end, in the final attempt.
    pub elapsed: Duration,
    /// Network paths used in the final attempt.
    pub paths_used: usize,
    /// Attempts needed.
    pub attempts: u32,
}

type ChunkKey = (u32, u32);

/// State shared by all paths of one attempt.
struct Attempt {
    files: Arc<Vec<SourceFile>>,
    handles: Vec<Arc<File>>,
    id: TransferId,
    queue: Mutex<VecDeque<ChunkKey>>,
    acked: Mutex<HashSet<ChunkKey>>,
    pending: Mutex<HashMap<ChunkKey, oneshot::Sender<bool>>>,
    outstanding: AtomicUsize,
    sent: AtomicU64,
    to_send: u64,
    paths: AtomicUsize,
    work: Notify,
    events: SendEvents,
}

impl Attempt {
    fn pop(&self) -> Option<ChunkKey> {
        lock(&self.queue).pop_front()
    }

    fn requeue(&self, key: ChunkKey) {
        if !lock(&self.acked).contains(&key) {
            lock(&self.queue).push_back(key);
            self.work.notify_waiters();
        }
    }

    fn is_finished(&self) -> bool {
        self.outstanding.load(Ordering::Acquire) == 0
    }

    fn chunk_len(&self, (file, chunk): ChunkKey) -> u64 {
        u64::from(self.files[file as usize].entry.chunk_len(chunk))
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Sends `files` (see [`crate::prepare`]) to the paired device `peer`.
pub async fn send_prepared(
    node: Arc<Node>,
    peer: DeviceId,
    addrs: Vec<SocketAddr>,
    files: Arc<Vec<SourceFile>>,
    options: SendOptions,
    events: SendEvents,
) -> Result<SendReport> {
    let handles = files
        .iter()
        .map(|f| {
            File::open(&f.local)
                .map(Arc::new)
                .map_err(|e| Error::io("open", &f.local, e))
        })
        .collect::<Result<Vec<_>>>()?;
    let id: TransferId = rand::random();
    let mut addrs = addrs;
    let mut last_error = Error::Interrupted("no attempt made".into());

    for attempt in 1..=options.max_attempts.max(1) {
        if attempt > 1 {
            events(SendEvent::Retrying {
                attempt,
                reason: last_error.to_string(),
            });
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Some(source) = &options.rediscover {
                let found = source().await;
                if !found.is_empty() {
                    addrs = found;
                }
            }
        }
        match run_attempt(&node, peer, &addrs, &files, &handles, id, &options, &events).await {
            Ok(mut report) => {
                report.attempts = attempt;
                return Ok(report);
            }
            Err(e) if is_retryable(&e) => last_error = e,
            Err(e) => return Err(e),
        }
    }
    Err(Error::Interrupted(last_error.to_string()))
}

/// Collects, hashes and sends the files under `paths`.
pub async fn send_paths(
    node: Arc<Node>,
    peer: DeviceId,
    addrs: Vec<SocketAddr>,
    paths: Vec<PathBuf>,
    options: SendOptions,
    events: SendEvents,
) -> Result<SendReport> {
    let files = tokio::task::spawn_blocking(move || crate::manifest::prepare(&paths))
        .await
        .map_err(|_| Error::Protocol("hashing task failed"))??;
    send_prepared(node, peer, addrs, Arc::new(files), options, events).await
}

fn is_retryable(e: &Error) -> bool {
    matches!(e, Error::Net(_) | Error::Interrupted(_))
}

#[allow(clippy::too_many_arguments)]
async fn run_attempt(
    node: &Arc<Node>,
    peer: DeviceId,
    addrs: &[SocketAddr],
    files: &Arc<Vec<SourceFile>>,
    handles: &[Arc<File>],
    id: TransferId,
    options: &SendOptions,
    events: &SendEvents,
) -> Result<SendReport> {
    let conn = node.connect(peer, addrs).await?;
    let (mut send, mut recv) = conn.open_bi(CONTROL_KIND).await?;
    write_offer(&mut send, id, files).await?;
    let needed = read_needs(&mut recv, files).await?;

    let to_send: u64 = needed
        .iter()
        .map(|(f, c)| u64::from(files[*f as usize].entry.chunk_len(*c)))
        .sum();
    let total: u64 = files.iter().map(|f| f.entry.size).sum();
    events(SendEvent::Accepted { to_send, total });

    let attempt = Arc::new(Attempt {
        files: files.clone(),
        handles: handles.to_vec(),
        id,
        outstanding: AtomicUsize::new(needed.len()),
        queue: Mutex::new(needed.into()),
        acked: Mutex::new(HashSet::new()),
        pending: Mutex::new(HashMap::new()),
        sent: AtomicU64::new(0),
        to_send,
        paths: AtomicUsize::new(0),
        work: Notify::new(),
        events: events.clone(),
    });
    let start = Instant::now();

    let mut tasks = tokio::task::JoinSet::new();
    let mut used: HashSet<SocketAddr> = HashSet::new();
    used.insert(conn.remote_addr());
    add_path(&mut tasks, &attempt, conn.clone(), options.streams_per_path);

    // Other addresses may be other networks (Wi-Fi and cable at once).
    for addr in addrs.iter().filter(|a| a.ip() != conn.remote_addr().ip()) {
        if let Ok(extra) = node.connect(peer, &[*addr]).await {
            used.insert(*addr);
            add_path(&mut tasks, &attempt, extra, options.streams_per_path);
        }
    }

    let acks = read_acks(recv, attempt.clone());
    tokio::pin!(acks);
    let mut interfaces = local_interfaces();
    let mut ticker = tokio::time::interval(PATH_CHECK_INTERVAL);
    ticker.tick().await;

    let paths_used;
    loop {
        tokio::select! {
            result = &mut acks => {
                result?;
                paths_used = used.len();
                break;
            }
            _ = ticker.tick(), if options.rediscover.is_some() => {
                // A new local interface (cable plugged in, network switched) may give a
                // new route to the peer.
                let now = local_interfaces();
                if now != interfaces {
                    interfaces = now;
                    if let Some(source) = &options.rediscover {
                        for addr in source().await {
                            if used.contains(&addr) {
                                continue;
                            }
                            if let Ok(extra) = node.connect(peer, &[addr]).await {
                                used.insert(addr);
                                add_path(&mut tasks, &attempt, extra, options.streams_per_path);
                            }
                        }
                    }
                }
            }
            Some(_) = tasks.join_next() => {}
        }
    }
    tasks.abort_all();
    let _ = send.finish();
    conn.close();

    Ok(SendReport {
        files: files.len(),
        total_bytes: total,
        bytes_sent: attempt.sent.load(Ordering::Acquire),
        elapsed: start.elapsed(),
        paths_used,
        attempts: 0,
    })
}

fn add_path(
    tasks: &mut tokio::task::JoinSet<()>,
    attempt: &Arc<Attempt>,
    conn: PeerConnection,
    workers: usize,
) {
    let addr = conn.remote_addr();
    attempt.paths.fetch_add(1, Ordering::AcqRel);
    (attempt.events)(SendEvent::PathAdded { addr });
    let alive = Arc::new(AtomicUsize::new(workers.max(1)));
    for _ in 0..workers.max(1) {
        let (attempt, conn, alive) = (attempt.clone(), conn.clone(), alive.clone());
        tasks.spawn(async move {
            worker(&attempt, &conn).await;
            if alive.fetch_sub(1, Ordering::AcqRel) == 1 {
                attempt.paths.fetch_sub(1, Ordering::AcqRel);
                if !attempt.is_finished() {
                    (attempt.events)(SendEvent::PathLost { addr });
                }
            }
        });
    }
}

/// Sends chunks over one connection until the queue is empty and everything is
/// acknowledged, or the connection fails.
async fn worker(attempt: &Arc<Attempt>, conn: &PeerConnection) {
    loop {
        if attempt.is_finished() {
            return;
        }
        let Some(key) = attempt.pop() else {
            // Nothing queued, but chunks are in flight elsewhere and may come back.
            let _ = timeout(Duration::from_millis(250), attempt.work.notified()).await;
            continue;
        };
        if lock(&attempt.acked).contains(&key) {
            continue;
        }
        let (tx, rx) = oneshot::channel();
        lock(&attempt.pending).insert(key, tx);

        match send_chunk(attempt, conn, key).await {
            Ok(()) => {}
            Err(_) => {
                lock(&attempt.pending).remove(&key);
                attempt.requeue(key);
                return;
            }
        }
        let outcome = tokio::select! {
            ack = timeout(ACK_TIMEOUT, rx) => ack.ok().and_then(|r| r.ok()),
            () = conn.closed() => None,
        };
        match outcome {
            Some(true) => {}
            Some(false) => attempt.requeue(key),
            None => {
                lock(&attempt.pending).remove(&key);
                attempt.requeue(key);
                if conn.quic().close_reason().is_some() {
                    return;
                }
            }
        }
    }
}

async fn send_chunk(attempt: &Attempt, conn: &PeerConnection, key: ChunkKey) -> Result<()> {
    let (file, chunk) = key;
    let source = &attempt.files[file as usize];
    let handle = attempt.handles[file as usize].clone();
    let len = source.entry.chunk_len(chunk) as usize;
    let offset = source.entry.chunk_offset(chunk);
    let local = source.local.clone();
    let data = tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; len];
        read_exact_at(&handle, &mut buf, offset).map_err(|e| Error::io("read", &local, e))?;
        Ok::<_, Error>(Bytes::from(buf))
    })
    .await
    .map_err(|_| Error::Protocol("read task failed"))??;

    let mut stream = conn.open_uni(CHUNK_KIND).await?;
    let header = ChunkHeader {
        id: attempt.id,
        file,
        chunk,
    };
    stream
        .write_all(&header.encode())
        .await
        .map_err(ec_net::Error::from)?;
    stream
        .write_chunk(data)
        .await
        .map_err(ec_net::Error::from)?;
    let _ = stream.finish();
    Ok(())
}

async fn write_offer(
    send: &mut ec_net::SendStream,
    id: TransferId,
    files: &[SourceFile],
) -> Result<()> {
    let total_bytes = files.iter().map(|f| f.entry.size).sum();
    write_msg(
        send,
        &Control::Offer {
            id,
            files: files.len() as u32,
            total_bytes,
        },
    )
    .await?;
    for (index, file) in files.iter().enumerate() {
        let entry = &file.entry;
        let header = Control::File {
            index: index as u32,
            path: entry.path.clone(),
            size: entry.size,
            chunk_size: entry.chunk_size,
        };
        write_msg(send, &header).await?;
        for (n, hashes) in entry.hashes.chunks(MAX_HASHES_PER_MSG).enumerate() {
            let msg = Control::Hashes {
                file: index as u32,
                start: (n * MAX_HASHES_PER_MSG) as u32,
                hashes: hashes.to_vec(),
            };
            write_msg(send, &msg).await?;
        }
    }
    write_msg(send, &Control::OfferEnd).await?;
    Ok(())
}

async fn read_needs(recv: &mut RecvStream, files: &[SourceFile]) -> Result<Vec<ChunkKey>> {
    let mut needed = Vec::new();
    loop {
        let msg = timeout(STEP_TIMEOUT, read_msg(recv))
            .await
            .map_err(|_| Error::Interrupted("receiver did not answer the offer".into()))??;
        match msg {
            Control::Need { file, ranges } => {
                let entry = &files
                    .get(file as usize)
                    .ok_or(Error::Protocol("need for unknown file"))?
                    .entry;
                for (start, end) in ranges {
                    if start >= end || end > entry.chunk_count() {
                        return Err(Error::Protocol("need range out of bounds"));
                    }
                    needed.extend((start..end).map(|c| (file, c)));
                }
            }
            Control::NeedEnd => return Ok(needed),
            Control::Rejected { reason } => return Err(Error::Rejected(reason)),
            _ => return Err(Error::Protocol("unexpected message before needs")),
        }
    }
}

/// Reads acknowledgements until the receiver reports that everything is saved.
async fn read_acks(mut recv: RecvStream, attempt: Arc<Attempt>) -> Result<()> {
    loop {
        match read_msg(&mut recv).await? {
            Control::Ack { file, chunk, ok } => {
                let key = (file, chunk);
                let waiter = lock(&attempt.pending).remove(&key);
                if ok && lock(&attempt.acked).insert(key) {
                    attempt.outstanding.fetch_sub(1, Ordering::AcqRel);
                    let sent = attempt
                        .sent
                        .fetch_add(attempt.chunk_len(key), Ordering::AcqRel)
                        + attempt.chunk_len(key);
                    (attempt.events)(SendEvent::Progress {
                        sent,
                        to_send: attempt.to_send,
                        paths: attempt.paths.load(Ordering::Acquire),
                    });
                    attempt.work.notify_waiters();
                }
                if let Some(waiter) = waiter {
                    let _ = waiter.send(ok);
                }
            }
            Control::FileDone { .. } => {}
            Control::Done => return Ok(()),
            _ => return Err(Error::Protocol("unexpected message during transfer")),
        }
    }
}

/// The set of this machine's IPv4 addresses, to notice interface changes.
fn local_interfaces() -> Vec<std::net::IpAddr> {
    ec_net::local_addrs()
}
