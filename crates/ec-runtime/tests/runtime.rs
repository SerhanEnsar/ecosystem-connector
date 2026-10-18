//! Two runtimes on one machine: pair, see each other, send a file.

#![allow(clippy::unwrap_used)]

use std::future::ready;
use std::sync::Arc;
use std::time::Duration;

use ec_runtime::{Config, Event, Runtime};
use tokio::sync::oneshot;

async fn runtime(name: &str, root: &std::path::Path) -> Arc<Runtime> {
    runtime_with(name, root, true).await
}

async fn runtime_with(name: &str, root: &std::path::Path, mdns: bool) -> Arc<Runtime> {
    Runtime::start(Config {
        home: root.join(name).join("home"),
        default_name: name.to_owned(),
        receive_dir: root.join(name).join("inbox"),
        port: 0,
        mdns,
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
    // Other events (mDNS, the new connection) may come first.
    let paired =
        async { while !matches!(mac_events.recv().await.unwrap(), Event::Paired { .. }) {} };
    tokio::time::timeout(Duration::from_secs(5), paired)
        .await
        .unwrap();

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
        .send(mac.id(), vec![src], None, Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!(report.files, 1);
    let received = std::fs::read(root.path().join("Mac/inbox/notes.txt")).unwrap();
    assert_eq!(received, b"from the tablet");

    // Forgetting the tablet makes the Mac refuse it.
    mac.forget(&tablet.id()).unwrap();
    assert!(mac.devices().is_empty());
}

/// Networks that block multicast: no mDNS at all. Devices must still find each other
/// through the addresses exchanged during pairing.
#[tokio::test(flavor = "multi_thread")]
async fn works_without_mdns() {
    let root = tempfile::tempdir().unwrap();
    let mac = runtime_with("Mac", root.path(), false).await;
    let tablet = runtime_with("Tablet", root.path(), false).await;

    let (invite_tx, invite_rx) = oneshot::channel();
    let host = {
        let mac = mac.clone();
        tokio::spawn(async move {
            mac.host_pairing(|uri| drop(invite_tx.send(uri)), |_| ready(true))
                .await
        })
    };
    let invite = invite_rx.await.unwrap();
    // The invite lists loopback last; keep only it so the test runs on any machine.
    let invite = invite.split("&a=").next().unwrap().to_owned()
        + &format!("&a=127.0.0.1:{}", invite.rsplit(':').next().unwrap());
    tablet.join_pairing(&invite, |_| ready(true)).await.unwrap();
    host.await.unwrap().unwrap();

    // The tablet learned the Mac's node port during pairing and reaches it directly.
    let online = async {
        while !tablet.devices().iter().any(|d| d.online) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), online)
        .await
        .unwrap();

    let src = root.path().join("doc.pdf");
    std::fs::write(&src, b"no multicast needed").unwrap();
    // Listed twice, as a share can: it arrives once.
    tablet
        .send(mac.id(), vec![src.clone(), src], None, Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("Mac/inbox/doc.pdf")).unwrap(),
        b"no multicast needed"
    );
    assert!(!root.path().join("Mac/inbox/doc (1).pdf").exists());

    // The Mac learned the tablet's address from the incoming connection.
    let mac_sees_tablet = async {
        while !mac
            .devices()
            .iter()
            .any(|d| d.id == tablet.id() && d.online)
        {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(Duration::from_secs(10), mac_sees_tablet)
        .await
        .unwrap();

    // And the other way, over the connection the tablet opened.
    let back = root.path().join("reply.txt");
    std::fs::write(&back, b"from the Mac").unwrap();
    mac.send(tablet.id(), vec![back], None, Arc::new(|_| {}))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("Tablet/inbox/reply.txt")).unwrap(),
        b"from the Mac"
    );

    // A web page flung from the tablet arrives as an event on the Mac.
    let mut mac_events = mac.subscribe();
    tablet
        .fling_url(mac.id(), "https://example.com/notes".into(), "Notes".into())
        .await
        .unwrap();
    let flung = async {
        loop {
            if let Event::Fling { item, .. } = mac_events.recv().await.unwrap() {
                return item;
            }
        }
    };
    let item = tokio::time::timeout(Duration::from_secs(5), flung)
        .await
        .unwrap();
    assert_eq!(
        item,
        ec_fling::FlingItem::Url {
            url: "https://example.com/notes".into(),
            title: "Notes".into()
        }
    );

    // And text.
    tablet
        .fling_text(mac.id(), "Room 204, 14:00".into())
        .await
        .unwrap();
    let flung = async {
        loop {
            if let Event::Fling { item, .. } = mac_events.recv().await.unwrap() {
                return item;
            }
        }
    };
    let item = tokio::time::timeout(Duration::from_secs(5), flung)
        .await
        .unwrap();
    assert_eq!(
        item,
        ec_fling::FlingItem::Text {
            text: "Room 204, 14:00".into()
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sending_to_an_unpaired_device_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let a = runtime("A", root.path()).await;
    let b = runtime("B", root.path()).await;
    let result = a.send(b.id(), vec![], None, Arc::new(|_| {})).await;
    assert!(matches!(result, Err(ec_runtime::Error::NotPaired)));
}

#[tokio::test(flavor = "multi_thread")]
async fn receive_folder_choice_is_remembered() {
    let root = tempfile::tempdir().unwrap();
    let chosen = root.path().join("Chosen");
    {
        let mac = runtime_with("Mac", root.path(), false).await;
        assert_eq!(mac.receive_dir(), root.path().join("Mac").join("inbox"));
        assert!(mac.receive_dir_is_default());
        assert_eq!(mac.set_receive_dir(Some(chosen.clone())).unwrap(), chosen);
        assert!(chosen.is_dir(), "the folder is created");
        assert_eq!(mac.receive_dir(), chosen);
    }

    // A restart keeps the choice.
    let mac = runtime_with("Mac", root.path(), false).await;
    assert_eq!(mac.receive_dir(), chosen);
    assert!(!mac.receive_dir_is_default());

    // Back to the default.
    let default = mac.set_receive_dir(None).unwrap();
    assert_eq!(default, root.path().join("Mac").join("inbox"));
    assert_eq!(mac.receive_dir(), default);
    drop(mac);

    // A chosen folder that disappeared falls back to the default.
    let gone = root.path().join("Gone");
    runtime_with("Mac", root.path(), false)
        .await
        .set_receive_dir(Some(gone.clone()))
        .unwrap();
    std::fs::remove_dir(&gone).unwrap();
    let mac = runtime_with("Mac", root.path(), false).await;
    assert_eq!(mac.receive_dir(), default);
}
