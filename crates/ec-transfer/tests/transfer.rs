//! End-to-end transfer tests over real QUIC connections.

#![allow(clippy::unwrap_used)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ec_core::DeviceIdentity;
use ec_net::{Node, Router, read_msg, serve_connection, write_msg};
use ec_transfer::proto::{CHUNK_KIND, CONTROL_KIND, ChunkHeader, Control};
use ec_transfer::{Receiver, SendEvent, SendEvents, SendOptions, SendReport, send_paths};
use serde::Serialize;

const LOCALHOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

struct Pair {
    sender_identity: Arc<DeviceIdentity>,
    receiver_identity: Arc<DeviceIdentity>,
    sender: Arc<Node>,
    port: u16,
    dir: tempfile::TempDir,
}

impl Pair {
    fn addr(&self) -> SocketAddr {
        SocketAddr::new(LOCALHOST, self.port)
    }

    /// A new node for the same sending device, as after an app restart.
    fn new_sender_node(&self) -> Arc<Node> {
        Arc::new(
            Node::bind(
                self.sender_identity.clone(),
                [self.receiver_identity.id()],
                SocketAddr::new(LOCALHOST, 0),
            )
            .unwrap(),
        )
    }

    async fn send_with(
        &self,
        node: Arc<Node>,
        paths: Vec<PathBuf>,
        options: SendOptions,
        events: SendEvents,
    ) -> ec_transfer::Result<SendReport> {
        send_paths(
            node,
            self.receiver_identity.id(),
            vec![self.addr()],
            paths,
            options,
            events,
        )
        .await
    }

    async fn send(&self, paths: Vec<PathBuf>) -> SendReport {
        self.send_with(
            self.sender.clone(),
            paths,
            SendOptions::default(),
            Arc::new(|_| {}),
        )
        .await
        .unwrap()
    }
}

/// A sender node and a receiving node (running a Receiver) that trust each other.
/// The receiver listens on `bind`.
async fn pair(bind: IpAddr) -> Pair {
    let a = Arc::new(DeviceIdentity::generate("Sender").unwrap());
    let b = Arc::new(DeviceIdentity::generate("Receiver").unwrap());
    let sender = Arc::new(Node::bind(a.clone(), [b.id()], SocketAddr::new(bind, 0)).unwrap());
    let receiver = Arc::new(Node::bind(b.clone(), [a.id()], SocketAddr::new(bind, 0)).unwrap());
    let dir = tempfile::tempdir().unwrap();

    let service = Arc::new(Receiver::new(
        dir.path().to_path_buf(),
        Arc::new(|_| true),
        Arc::new(|_| {}),
    ));
    let mut router = Router::with_session("Receiver", |_, _| {});
    router.add(CONTROL_KIND, service.clone()).unwrap();
    router.add(CHUNK_KIND, service).unwrap();
    let router = Arc::new(router);
    let port = receiver.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Some(conn) = receiver.accept().await {
            if let Ok(conn) = conn {
                tokio::spawn(serve_connection(conn, router.clone()));
            }
        }
    });
    Pair {
        sender_identity: a,
        receiver_identity: b,
        sender,
        port,
        dir,
    }
}

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    let mut out = vec![0u8; len];
    blake3::Hasher::new_keyed(&[seed; 32])
        .finalize_xof()
        .fill(&mut out);
    out
}

fn read(dir: &Path, rel: &str) -> Vec<u8> {
    std::fs::read(dir.join(rel)).unwrap()
}

#[tokio::test]
async fn sends_a_folder_with_large_small_and_empty_files() {
    let pair = pair(LOCALHOST).await;
    let src = tempfile::tempdir().unwrap();
    let folder = src.path().join("Project");
    std::fs::create_dir_all(folder.join("notes")).unwrap();
    let big = pattern(9 * 1024 * 1024 + 123, 1);
    std::fs::write(folder.join("video.bin"), &big).unwrap();
    std::fs::write(folder.join("notes/todo.txt"), b"ship it").unwrap();
    std::fs::write(folder.join("empty"), b"").unwrap();

    let report = pair.send(vec![folder]).await;
    assert_eq!(report.files, 3);
    assert_eq!(report.bytes_sent, report.total_bytes);
    let out = pair.dir.path();
    assert_eq!(read(out, "Project/video.bin"), big);
    assert_eq!(read(out, "Project/notes/todo.txt"), b"ship it");
    assert!(read(out, "Project/empty").is_empty());
    let partial = out.join(ec_transfer::PARTIAL_DIR);
    assert_eq!(
        std::fs::read_dir(partial).unwrap().count(),
        0,
        "partials cleaned up"
    );
}

#[tokio::test]
async fn sending_twice_never_overwrites() {
    let pair = pair(LOCALHOST).await;
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("report.pdf");
    std::fs::write(&file, b"v1").unwrap();
    pair.send(vec![file.clone()]).await;
    std::fs::write(&file, b"v2").unwrap();
    pair.send(vec![file]).await;
    assert_eq!(read(pair.dir.path(), "report.pdf"), b"v1");
    assert_eq!(read(pair.dir.path(), "report (1).pdf"), b"v2");
}

#[tokio::test]
async fn interrupted_transfer_resumes_where_it_stopped() {
    let pair = pair(LOCALHOST).await;
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("big.bin");
    let data = pattern(48 * 1024 * 1024, 2);
    std::fs::write(&file, &data).unwrap();

    // First attempt: cut the connection once some chunks are acknowledged.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let events: SendEvents = Arc::new(move |e| {
        if let SendEvent::Progress { sent, .. } = e
            && sent >= 12 * 1024 * 1024
        {
            let _ = tx.send(());
        }
    });
    let options = SendOptions {
        streams_per_path: 2,
        max_attempts: 1,
        rediscover: None,
    };
    let first = {
        let node = pair.sender.clone();
        let (peer, addr, paths) = (pair.receiver_identity.id(), pair.addr(), vec![file.clone()]);
        tokio::spawn(
            async move { send_paths(node, peer, vec![addr], paths, options, events).await },
        )
    };
    rx.recv().await.unwrap();
    first.abort();
    pair.sender.shutdown().await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Second attempt from a fresh node of the same device sends only what is missing.
    let report = pair
        .send_with(
            pair.new_sender_node(),
            vec![file],
            SendOptions::default(),
            Arc::new(|_| {}),
        )
        .await
        .unwrap();
    assert!(
        report.bytes_sent < report.total_bytes,
        "resumed: {report:?}"
    );
    assert!(report.bytes_sent > 0);
    assert_eq!(read(pair.dir.path(), "big.bin"), data);
}

#[tokio::test]
async fn uses_two_paths_when_available() {
    // Needs a non-loopback address to have a second path.
    let Some(lan) = ec_net::local_addrs()
        .into_iter()
        .find(|ip| !ip.is_loopback())
    else {
        eprintln!("skipped: no non-loopback IPv4 address");
        return;
    };
    let pair = pair(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).await;
    let src = tempfile::tempdir().unwrap();
    let file = src.path().join("data.bin");
    let data = pattern(32 * 1024 * 1024, 3);
    std::fs::write(&file, &data).unwrap();

    let addrs = vec![pair.addr(), SocketAddr::new(lan, pair.port)];
    let report = send_paths(
        pair.sender.clone(),
        pair.receiver_identity.id(),
        addrs,
        vec![file],
        SendOptions::default(),
        Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert_eq!(report.paths_used, 2);
    assert_eq!(read(pair.dir.path(), "data.bin"), data);
}

/// Opens a raw control stream to the receiver, as a misbehaving peer would.
async fn raw_control(
    pair: &Pair,
) -> (
    ec_net::PeerConnection,
    ec_net::SendStream,
    ec_net::RecvStream,
) {
    let conn = pair
        .sender
        .connect(pair.receiver_identity.id(), &[pair.addr()])
        .await
        .unwrap();
    let (send, recv) = conn.open_bi(CONTROL_KIND).await.unwrap();
    (conn, send, recv)
}

#[tokio::test]
async fn corrupted_chunk_is_rejected_and_can_be_resent() {
    let pair = pair(LOCALHOST).await;
    let data = b"hello, verified world".to_vec();
    let id = [5u8; 16];
    let (conn, mut send, mut recv) = raw_control(&pair).await;

    for msg in [
        Control::Offer {
            id,
            files: 1,
            total_bytes: data.len() as u64,
        },
        Control::File {
            index: 0,
            path: ec_transfer::RelPath::new(vec!["hello.txt".into()]).unwrap(),
            size: data.len() as u64,
            chunk_size: 4 * 1024 * 1024,
        },
        Control::Hashes {
            file: 0,
            start: 0,
            hashes: vec![*blake3::hash(&data).as_bytes()],
        },
        Control::OfferEnd,
    ] {
        write_msg(&mut send, &msg).await.unwrap();
    }
    assert!(matches!(
        read_msg(&mut recv).await.unwrap(),
        Control::Need { .. }
    ));
    assert_eq!(
        read_msg::<Control>(&mut recv).await.unwrap(),
        Control::NeedEnd
    );

    let header = ChunkHeader {
        id,
        file: 0,
        chunk: 0,
    }
    .encode();
    for (body, expect_ok) in [
        (b"hello, tampered world".to_vec(), false),
        (data.clone(), true),
    ] {
        let mut chunk = conn.open_uni(CHUNK_KIND).await.unwrap();
        chunk.write_all(&header).await.unwrap();
        chunk.write_all(&body).await.unwrap();
        chunk.finish().unwrap();
        let ack = read_msg::<Control>(&mut recv).await.unwrap();
        assert_eq!(
            ack,
            Control::Ack {
                file: 0,
                chunk: 0,
                ok: expect_ok
            }
        );
    }
    assert_eq!(
        read_msg::<Control>(&mut recv).await.unwrap(),
        Control::FileDone { file: 0 }
    );
    assert_eq!(read_msg::<Control>(&mut recv).await.unwrap(), Control::Done);
    assert_eq!(read(pair.dir.path(), "hello.txt"), data);
}

/// Mirrors the first variants of `Control`, but with an unchecked path, to craft
/// offers the real types refuse to build.
#[derive(Serialize)]
#[allow(dead_code)]
enum EvilControl {
    Offer {
        id: [u8; 16],
        files: u32,
        total_bytes: u64,
    },
    File {
        index: u32,
        path: Vec<String>,
        size: u64,
        chunk_size: u32,
    },
}

#[tokio::test]
async fn path_traversal_in_an_offer_is_refused() {
    let pair = pair(LOCALHOST).await;
    let (_conn, mut send, mut recv) = raw_control(&pair).await;
    let evil_path = vec!["..".to_owned(), "..".to_owned(), "escaped.txt".to_owned()];
    write_msg(
        &mut send,
        &EvilControl::Offer {
            id: [6; 16],
            files: 1,
            total_bytes: 1,
        },
    )
    .await
    .unwrap();
    write_msg(
        &mut send,
        &EvilControl::File {
            index: 0,
            path: evil_path,
            size: 1,
            chunk_size: 4 * 1024 * 1024,
        },
    )
    .await
    .unwrap();
    // The receiver drops the stream without answering.
    assert!(read_msg::<Control>(&mut recv).await.is_err());
    let escaped = pair.dir.path().parent().unwrap().join("escaped.txt");
    assert!(!escaped.exists());
}
