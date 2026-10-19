//! Linked files between real runtimes on one machine: two or three devices, paired
//! over loopback without mDNS.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::future::ready;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ec_runtime::{Config, Event, LinkedConfig, Runtime};
use ec_sync::{LinkId, State, SyncEvent};
use ec_transfer::ReceiveEvent;
use tokio::sync::{broadcast, oneshot};

const WAIT: Duration = Duration::from_secs(30);
const MIB: usize = 1024 * 1024;

async fn device(name: &str, root: &Path, linked: LinkedConfig) -> Arc<Runtime> {
    let runtime = Runtime::start(Config {
        home: root.join(name).join("home"),
        default_name: name.to_owned(),
        receive_dir: root.join(name).join("inbox"),
        port: 0,
        mdns: false,
        linked,
    })
    .await
    .unwrap();
    // Check files often, as the Android app does, so tests do not wait for the backstop.
    runtime.set_linked_check_interval(Duration::from_millis(300));
    runtime
}

/// Pairs `host` and `guest` over loopback and waits until each sees the other online
/// with linked files (or, for an older device, just online).
async fn pair(host: &Arc<Runtime>, guest: &Arc<Runtime>) {
    let (invite_tx, invite_rx) = oneshot::channel();
    let hosting = {
        let host = host.clone();
        tokio::spawn(async move {
            host.host_pairing(|uri| drop(invite_tx.send(uri)), |_| ready(true))
                .await
        })
    };
    let invite = invite_rx.await.unwrap();
    let invite = invite.split("&a=").next().unwrap().to_owned()
        + &format!("&a=127.0.0.1:{}", invite.rsplit(':').next().unwrap());
    guest.join_pairing(&invite, |_| ready(true)).await.unwrap();
    hosting.await.unwrap().unwrap();
    wait_until(|| sees(host, guest) && sees(guest, host)).await;
}

fn sees(a: &Runtime, b: &Runtime) -> bool {
    a.devices().iter().any(|d| d.id == b.id() && d.online)
}

fn offers_linked(a: &Runtime, b: &Runtime) -> bool {
    a.devices().iter().any(|d| d.id == b.id() && d.linked_files)
}

async fn wait_until(check: impl Fn() -> bool) {
    tokio::time::timeout(WAIT, async {
        while !check() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("timed out");
}

/// Waits for a linked files event matching `want`.
async fn sync_event(
    events: &mut broadcast::Receiver<Event>,
    want: impl Fn(&SyncEvent) -> bool,
) -> SyncEvent {
    tokio::time::timeout(WAIT, async {
        loop {
            match events.recv().await {
                Ok(Event::Sync(event)) if want(&event) => return event,
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(e) => panic!("{e}"),
            }
        }
    })
    .await
    .expect("timed out waiting for a linked files event")
}

fn pattern(len: usize, seed: u8) -> Vec<u8> {
    let mut out = vec![0u8; len];
    blake3::Hasher::new_keyed(&[seed; 32])
        .finalize_xof()
        .fill(&mut out);
    out
}

fn append(path: &Path, data: &[u8]) {
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(data).unwrap();
}

fn linked_path(runtime: &Runtime, id: LinkId) -> PathBuf {
    runtime
        .linked_files()
        .into_iter()
        .find(|f| f.id == id)
        .unwrap()
        .path
}

fn state(runtime: &Runtime, id: LinkId) -> State {
    runtime
        .linked_files()
        .into_iter()
        .find(|f| f.id == id)
        .map(|f| f.state)
        .unwrap()
}

/// Mac and tablet, paired, with `name` linked from the Mac. Returns the link id.
async fn linked_pair(
    root: &Path,
    tablet_linked: LinkedConfig,
    name: &str,
    contents: &[u8],
) -> (Arc<Runtime>, Arc<Runtime>, LinkId, PathBuf) {
    let mac = device("Mac", root, LinkedConfig::default()).await;
    let tablet = device("Tablet", root, tablet_linked).await;
    pair(&mac, &tablet).await;
    wait_until(|| offers_linked(&mac, &tablet) && offers_linked(&tablet, &mac)).await;

    let file = root.join("Documents").join(name);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, contents).unwrap();
    let mut tablet_events = tablet.subscribe();
    let id = mac
        .send_linked(tablet.id(), file.clone(), false)
        .await
        .unwrap();
    sync_event(&mut tablet_events, |e| {
        matches!(e, SyncEvent::Linked { .. })
    })
    .await;
    (mac, tablet, id, file)
}

#[tokio::test(flavor = "multi_thread")]
async fn link_then_edit_sends_only_changed_chunks() {
    let root = tempfile::tempdir().unwrap();
    let first = pattern(12 * MIB, 1);
    let (mac, tablet, id, file) =
        linked_pair(root.path(), LinkedConfig::default(), "video.bin", &first).await;

    let at = linked_path(&tablet, id);
    let today = at
        .parent()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        at,
        root.path()
            .join("Tablet/inbox/Linked")
            .join(&today)
            .join("video.bin")
    );
    assert_eq!(std::fs::read(&at).unwrap(), first);
    assert!(!root.path().join("Tablet/inbox/video.bin").exists());

    let mut tablet_events = tablet.subscribe();
    append(&file, &pattern(MIB, 2));
    let mut reused = None;
    tokio::time::timeout(WAIT, async {
        loop {
            match tablet_events.recv().await.unwrap() {
                Event::Receive(ReceiveEvent::Started { already_have, .. }) => {
                    reused = Some(already_have);
                }
                Event::Sync(SyncEvent::Updated { id: updated }) if updated == id => break,
                _ => {}
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read(&at).unwrap(), std::fs::read(&file).unwrap());
    // The three unchanged 4 MiB chunks came from the tablet's own copy.
    assert_eq!(reused, Some(12 * MIB as u64));

    // The tablet does not send the update back.
    let mut mac_events = mac.subscribe();
    let echo = tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if let Ok(Event::Receive(_) | Event::Sync(SyncEvent::Updated { .. })) =
                mac_events.recv().await
            {
                return;
            }
        }
    })
    .await;
    assert!(echo.is_err(), "the update came back to the Mac");
}

#[tokio::test(flavor = "multi_thread")]
async fn offline_edits_on_both_sides_conflict_and_resolve() {
    let root = tempfile::tempdir().unwrap();
    let (mac, tablet, id, file) =
        linked_pair(root.path(), LinkedConfig::default(), "notes.txt", b"v1").await;
    let at = linked_path(&tablet, id);
    let tablet_id = tablet.id();

    tablet.shutdown().await;
    drop(tablet);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut mac_events = mac.subscribe();
    std::fs::write(&file, b"edited on the Mac").unwrap();
    std::fs::write(&at, b"edited on the tablet").unwrap();
    // The Mac notices its own edit (the tablet is away).
    tokio::time::sleep(Duration::from_secs(4)).await;

    let tablet = device("Tablet", root.path(), LinkedConfig::default()).await;
    assert_eq!(tablet.id(), tablet_id);
    let mut tablet_events = tablet.subscribe();
    sync_event(&mut tablet_events, |e| {
        matches!(e, SyncEvent::Conflict { .. })
    })
    .await;
    sync_event(&mut mac_events, |e| matches!(e, SyncEvent::Conflict { .. })).await;
    assert!(matches!(state(&tablet, id), State::Conflict { .. }));
    assert!(matches!(state(&mac, id), State::Conflict { .. }));
    // Neither edit was lost.
    assert_eq!(std::fs::read(&at).unwrap(), b"edited on the tablet");
    assert_eq!(std::fs::read(&file).unwrap(), b"edited on the Mac");

    tablet.resolve_conflict(id, true).unwrap();
    assert_eq!(std::fs::read(&at).unwrap(), b"edited on the Mac");
    wait_until(|| state(&mac, id) == State::Current && state(&tablet, id) == State::Current).await;
    assert_eq!(std::fs::read(&file).unwrap(), b"edited on the Mac");
    assert!(mac.linked_conflict_path(id).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn local_edit_during_update_becomes_conflict() {
    let root = tempfile::tempdir().unwrap();
    let armed = Arc::new(AtomicBool::new(false));
    let trigger = armed.clone();
    let linked = LinkedConfig {
        before_commit: Some(Arc::new(move |path: &Path| {
            if trigger.swap(false, Ordering::SeqCst) {
                std::fs::write(path, b"typed on the tablet meanwhile").unwrap();
            }
        })),
        ..LinkedConfig::default()
    };
    let (_mac, tablet, id, file) = linked_pair(root.path(), linked, "draft.txt", b"v1").await;
    let at = linked_path(&tablet, id);

    let mut tablet_events = tablet.subscribe();
    armed.store(true, Ordering::SeqCst);
    std::fs::write(&file, b"v2 from the Mac").unwrap();
    sync_event(&mut tablet_events, |e| {
        matches!(e, SyncEvent::Conflict { .. })
    })
    .await;
    assert_eq!(
        std::fs::read(&at).unwrap(),
        b"typed on the tablet meanwhile"
    );
    let waiting = tablet.linked_conflict_path(id).unwrap();
    assert_eq!(std::fs::read(waiting).unwrap(), b"v2 from the Mac");
}

#[tokio::test(flavor = "multi_thread")]
async fn move_keeps_syncing_and_delete_leaves_link() {
    let root = tempfile::tempdir().unwrap();
    let (mac, tablet, id, file) =
        linked_pair(root.path(), LinkedConfig::default(), "plan.txt", b"v1").await;
    let at = linked_path(&tablet, id);

    let mut tablet_events = tablet.subscribe();
    let moved = root.path().join("Tablet/inbox/Linked/Work/plan-final.txt");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&at, &moved).unwrap();
    sync_event(&mut tablet_events, |e| matches!(e, SyncEvent::Moved { .. })).await;
    assert_eq!(linked_path(&tablet, id), moved);

    std::fs::write(&file, b"v2 from the Mac").unwrap();
    sync_event(&mut tablet_events, |e| {
        matches!(e, SyncEvent::Updated { .. })
    })
    .await;
    assert_eq!(std::fs::read(&moved).unwrap(), b"v2 from the Mac");

    let mut mac_events = mac.subscribe();
    std::fs::remove_file(&moved).unwrap();
    sync_event(&mut mac_events, |e| {
        matches!(e, SyncEvent::PeerUnlinked { .. })
    })
    .await;
    assert_eq!(std::fs::read(&file).unwrap(), b"v2 from the Mac");
    assert!(tablet.linked_files().is_empty());
    assert!(mac.linked_files()[0].peers.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn three_devices_stay_current() {
    let root = tempfile::tempdir().unwrap();
    let (mac, tablet, id, file) =
        linked_pair(root.path(), LinkedConfig::default(), "list.txt", b"v1").await;
    let phone = device("Phone", root.path(), LinkedConfig::default()).await;
    pair(&mac, &phone).await;
    wait_until(|| offers_linked(&mac, &phone)).await;
    let mut phone_events = phone.subscribe();
    assert_eq!(
        mac.send_linked(phone.id(), file.clone(), false)
            .await
            .unwrap(),
        id
    );
    sync_event(&mut phone_events, |e| matches!(e, SyncEvent::Linked { .. })).await;

    // The phone only knows the Mac; the Mac passes the change on to the tablet.
    let mut mac_events = mac.subscribe();
    let mut tablet_events = tablet.subscribe();
    std::fs::write(linked_path(&phone, id), b"v2 from the phone").unwrap();
    sync_event(&mut mac_events, |e| matches!(e, SyncEvent::Updated { .. })).await;
    sync_event(&mut tablet_events, |e| {
        matches!(e, SyncEvent::Updated { .. })
    })
    .await;
    assert_eq!(std::fs::read(&file).unwrap(), b"v2 from the phone");
    assert_eq!(
        std::fs::read(linked_path(&tablet, id)).unwrap(),
        b"v2 from the phone"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn older_peer_is_not_offered_linked() {
    let root = tempfile::tempdir().unwrap();
    let mac = device("Mac", root.path(), LinkedConfig::default()).await;
    let old = device(
        "Old",
        root.path(),
        LinkedConfig {
            enabled: false,
            ..LinkedConfig::default()
        },
    )
    .await;
    let mut mac_events = mac.subscribe();
    pair(&mac, &old).await;
    let support = sync_event(&mut mac_events, |e| matches!(e, SyncEvent::Support { .. })).await;
    assert_eq!(
        support,
        SyncEvent::Support {
            peer: old.id(),
            supported: false
        }
    );
    assert!(!offers_linked(&mac, &old));
}

#[tokio::test(flavor = "multi_thread")]
async fn leaving_while_the_other_is_away_does_not_send_the_file_again() {
    let root = tempfile::tempdir().unwrap();
    let (mac, tablet, id, file) =
        linked_pair(root.path(), LinkedConfig::default(), "Makale.pdf", b"v1").await;
    let mac_id = mac.id();

    // The Mac goes away; the tablet leaves the link meanwhile, so its notice is lost.
    mac.shutdown().await;
    drop(mac);
    tokio::time::sleep(Duration::from_millis(500)).await;
    tablet.leave_link(id).unwrap();
    assert!(tablet.linked_files().is_empty());

    let mut tablet_events = tablet.subscribe();
    let mac = device("Mac", root.path(), LinkedConfig::default()).await;
    assert_eq!(mac.id(), mac_id);
    let mut mac_events = mac.subscribe();
    sync_event(&mut mac_events, |e| {
        matches!(e, SyncEvent::PeerUnlinked { .. })
    })
    .await;
    assert!(mac.linked_files()[0].peers.is_empty());
    // The Mac's next edit does not reach the tablet as a new file.
    std::fs::write(&file, b"v2 on the Mac").unwrap();
    let arrived = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(Event::Sync(SyncEvent::Linked { .. })) = tablet_events.recv().await {
                return;
            }
        }
    })
    .await;
    assert!(
        arrived.is_err(),
        "the file came back to the tablet as a new link"
    );
}

// Windows reports a file moved out of its folder as removed, so there it is a delete
// (a known limit in the spec), not missing.
#[cfg(not(windows))]
#[tokio::test(flavor = "multi_thread")]
async fn a_missing_file_is_reported_to_the_other_device() {
    let root = tempfile::tempdir().unwrap();
    let (mac, tablet, id, _file) =
        linked_pair(root.path(), LinkedConfig::default(), "Not.txt", b"v1").await;
    let at = linked_path(&tablet, id);
    let mut mac_events = mac.subscribe();
    // Gone without a delete event: moved somewhere the watcher cannot see.
    let away = root.path().join("elsewhere");
    std::fs::create_dir_all(&away).unwrap();
    std::fs::rename(&at, away.join("Not.txt")).unwrap();
    let event = sync_event(&mut mac_events, |e| {
        matches!(e, SyncEvent::PeerMissing { .. })
    })
    .await;
    assert_eq!(
        event,
        SyncEvent::PeerMissing {
            id,
            peer: tablet.id(),
            missing: true
        }
    );
    assert_eq!(mac.linked_missing_on(id), vec![tablet.id()]);
}
