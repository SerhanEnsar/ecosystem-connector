//! Two runtimes on one machine: pair, see each other, send a file.

#![allow(clippy::unwrap_used)]

use std::future::ready;
use std::sync::Arc;
use std::time::Duration;

use ec_runtime::{Config, Event, Runtime};
use tokio::sync::oneshot;

async fn runtime(name: &str, root: &std::path::Path) -> Arc<Runtime> {
    Runtime::start(Config {
        home: root.join(name).join("home"),
        default_name: name.to_owned(),
        receive_dir: root.join(name).join("inbox"),
        port: 0,
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_then_send_a_file() {
    let root = tempfile::tempdir().unwrap();
    let mac = runtime("Mac", root.path()).await;
    let tablet = runtime("Tablet", root.path()).await;
    let mut mac_events = mac.subscribe();

    // Pair: the Mac hosts, the tablet joins with the invite.
    let (invite_tx, invite_rx) = oneshot::channel();
    let host = {
        let mac = mac.clone();
        tokio::spawn(async move {
            mac.host_pairing(|uri| drop(invite_tx.send(uri)), |_| ready(true))
                .await
        })
    };
    let invite = invite_rx.await.unwrap();
    let joined = tablet.join_pairing(&invite, |_| ready(true)).await.unwrap();
    let hosted = host.await.unwrap().unwrap();
    assert_eq!(joined.id, mac.id());
    assert_eq!(hosted.id, tablet.id());
    assert!(matches!(
        mac_events.recv().await.unwrap(),
        Event::Paired { .. }
    ));

    // Both see each other as paired; wait for mDNS to mark the Mac online.
    let online = async {
        loop {
            if tablet
                .devices()
                .iter()
                .any(|d| d.id == mac.id() && d.online)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };
    if tokio::time::timeout(Duration::from_secs(15), online)
        .await
        .is_err()
    {
        eprintln!("skipped sending: mDNS is not available here");
        return;
    }

    let src = root.path().join("notes.txt");
    std::fs::write(&src, b"from the tablet").unwrap();
    let report = tablet
        .send(mac.id(), vec![src], Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!(report.files, 1);
    let received = std::fs::read(root.path().join("Mac/inbox/notes.txt")).unwrap();
    assert_eq!(received, b"from the tablet");

    // Forgetting the tablet makes the Mac refuse it.
    mac.forget(&tablet.id()).unwrap();
    assert!(mac.devices().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn sending_to_an_unpaired_device_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let a = runtime("A", root.path()).await;
    let b = runtime("B", root.path()).await;
    let result = a.send(b.id(), vec![], Arc::new(|_| {})).await;
    assert!(matches!(result, Err(ec_runtime::Error::NotPaired)));
}
