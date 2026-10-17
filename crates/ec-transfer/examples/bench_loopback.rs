//! Loopback benchmark: hashing speed and transfer throughput by stream count.
//!
//! Both devices run in this process and talk over 127.0.0.1, with real QUIC, TLS,
//! hashing and disk writes. The result is an upper bound set by CPU and disk, not a
//! Wi-Fi measurement.
//!
//!     cargo run --release -p ec-transfer --example bench_loopback -- [size_mib] [runs]
//!
//! Writes CSV to bench/out/loopback.csv.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use ec_core::DeviceIdentity;
use ec_net::{Node, Router, serve_connection};
use ec_transfer::{CHUNK_KIND, CONTROL_KIND, Receiver, SendOptions, prepare, send_prepared};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let size_mib: usize = args
        .next()
        .map(|a| a.parse().expect("size in MiB"))
        .unwrap_or(1024);
    let runs: usize = args.next().map(|a| a.parse().expect("runs")).unwrap_or(3);

    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("payload.bin");
    {
        let mut out = std::fs::File::create(&file).unwrap();
        let mut reader = blake3::Hasher::new_keyed(&[42; 32]).finalize_xof();
        let mut buf = vec![0u8; 1024 * 1024];
        for _ in 0..size_mib {
            reader.fill(&mut buf);
            out.write_all(&buf).unwrap();
        }
        out.sync_all().unwrap();
    }
    let bytes = (size_mib * 1024 * 1024) as f64;

    let start = Instant::now();
    let files = Arc::new(prepare(std::slice::from_ref(&file)).unwrap());
    let hash_mbps = bytes / start.elapsed().as_secs_f64() / 1e6;
    println!("hashing: {hash_mbps:.0} MB/s");

    let a = Arc::new(DeviceIdentity::generate("Sender").unwrap());
    let b = Arc::new(DeviceIdentity::generate("Receiver").unwrap());
    let local = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let sender = Arc::new(Node::bind(a.clone(), [b.id()], local).unwrap());
    let receiver = Arc::new(Node::bind(b.clone(), [a.id()], local).unwrap());
    let addr = receiver.local_addr().unwrap();

    let mut rows = vec![format!(
        "kind,streams,run,mb_per_s\nhash,0,0,{hash_mbps:.1}"
    )];
    for streams in [1usize, 2, 4, 8, 16] {
        let mut speeds = Vec::new();
        for run in 0..runs {
            let out = tempfile::tempdir_in(work.path()).unwrap();
            let service = Arc::new(Receiver::new(
                out.path().to_path_buf(),
                Arc::new(|_| true),
                Arc::new(|_| {}),
            ));
            let mut router = Router::with_session("Receiver", |_, _| {});
            router.add(CONTROL_KIND, service.clone()).unwrap();
            router.add(CHUNK_KIND, service).unwrap();
            let router = Arc::new(router);
            let node = receiver.clone();
            let server = tokio::spawn(async move {
                while let Some(Ok(conn)) = node.accept().await {
                    tokio::spawn(serve_connection(conn, router.clone()));
                }
            });

            let options = SendOptions {
                streams_per_path: streams,
                ..SendOptions::default()
            };
            let report = send_prepared(
                sender.clone(),
                b.id(),
                vec![addr],
                files.clone(),
                options,
                Arc::new(|_| {}),
            )
            .await
            .unwrap();
            let mbps = report.bytes_sent as f64 / report.elapsed.as_secs_f64() / 1e6;
            println!("streams {streams:>2}, run {run}: {mbps:.0} MB/s");
            rows.push(format!("transfer,{streams},{run},{mbps:.1}"));
            speeds.push(mbps);
            server.abort();
        }
    }

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bench/out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let csv = out_dir.join("loopback.csv");
    std::fs::write(&csv, rows.join("\n") + "\n").unwrap();
    println!("wrote {}", csv.display());
}
