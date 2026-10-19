//! The watcher on a real folder: what it reports, and that it does not hash files whose
//! size and modification time did not change.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use ec_sync::watch::{WatchConfig, WatchEvent, Watched, Watcher, describe};
use ec_sync::{FileIdentity, LinkId};
use tokio::sync::mpsc;

const ID: LinkId = [3; 16];

struct Setup {
    _dir: tempfile::TempDir,
    root: PathBuf,
    path: PathBuf,
    watcher: Watcher,
    events: mpsc::UnboundedReceiver<WatchEvent>,
}

fn config(root: &Path) -> WatchConfig {
    WatchConfig {
        quiet: Duration::from_millis(200),
        stat_interval: Duration::from_millis(200),
        roots: vec![root.to_path_buf()],
    }
}

fn start(file: Option<Watched>) -> Setup {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("Linked");
    std::fs::create_dir_all(root.join("2026-10-19")).unwrap();
    let path = root.join("2026-10-19").join("Rapor.pdf");
    std::fs::write(&path, b"first version").unwrap();
    let file = file.unwrap_or_else(|| describe(ID, &path).unwrap());
    let (tx, events) = mpsc::unbounded_channel();
    let watcher = Watcher::start(config(&root), vec![file], tx).unwrap();
    Setup {
        _dir: dir,
        root,
        path,
        watcher,
        events,
    }
}

async fn next(
    events: &mut mpsc::UnboundedReceiver<WatchEvent>,
    within: Duration,
) -> Option<WatchEvent> {
    tokio::time::timeout(within, events.recv())
        .await
        .ok()
        .flatten()
}

async fn nothing_for(events: &mut mpsc::UnboundedReceiver<WatchEvent>, during: Duration) {
    if let Some(event) = next(events, during).await {
        panic!("unexpected {event:?}");
    }
}

#[tokio::test]
async fn unchanged_file_is_never_hashed() {
    let mut s = start(None);
    nothing_for(&mut s.events, Duration::from_secs(1)).await;
    assert_eq!(s.watcher.hash_count(), 0);
}

#[tokio::test]
async fn touch_hashes_once_and_reports_nothing() {
    let mut s = start(None);
    let file = std::fs::File::options().write(true).open(&s.path).unwrap();
    file.set_modified(SystemTime::now() + Duration::from_secs(10))
        .unwrap();
    drop(file);
    nothing_for(&mut s.events, Duration::from_millis(1500)).await;
    assert_eq!(s.watcher.hash_count(), 1);
}

#[tokio::test]
async fn edit_is_reported_once() {
    let mut s = start(None);
    std::fs::write(&s.path, b"first version, and more").unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Changed { id, content, .. }) => {
            assert_eq!(id, ID);
            assert_eq!(content.size, 23);
        }
        other => panic!("expected a change, got {other:?}"),
    }
    nothing_for(&mut s.events, Duration::from_secs(1)).await;
}

#[tokio::test]
async fn own_write_is_not_a_change() {
    let mut s = start(None);
    std::fs::write(&s.path, b"received from the Mac").unwrap();
    let now = describe(ID, &s.path).unwrap();
    s.watcher.expect_write(ID, now.seen, now.content);
    nothing_for(&mut s.events, Duration::from_millis(1500)).await;
}

#[tokio::test]
async fn atomic_save_by_rename_is_a_change() {
    let mut s = start(None);
    let tmp = s.path.with_file_name(".Rapor.pdf.sb-tmp");
    std::fs::write(&tmp, b"saved by an editor").unwrap();
    std::fs::rename(&tmp, &s.path).unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Changed { content, .. }) => assert_eq!(content.size, 18),
        other => panic!("expected a change, got {other:?}"),
    }
    nothing_for(&mut s.events, Duration::from_secs(1)).await;
}

#[tokio::test]
async fn move_inside_root_is_followed() {
    let mut s = start(None);
    let to = s.root.join("Work").join("Rapor-final.pdf");
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::rename(&s.path, &to).unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Moved { id, to: at, .. }) => {
            assert_eq!(id, ID);
            assert_eq!(at, to);
        }
        other => panic!("expected a move, got {other:?}"),
    }
    // It is watched at the new place.
    std::fs::write(&to, b"edited after the move").unwrap();
    assert!(matches!(
        next(&mut s.events, Duration::from_secs(5)).await,
        Some(WatchEvent::Changed { .. })
    ));
}

#[tokio::test]
async fn delete_is_reported() {
    let mut s = start(None);
    std::fs::remove_file(&s.path).unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Deleted { id }) => assert_eq!(id, ID),
        other => panic!("expected a delete, got {other:?}"),
    }
}

#[tokio::test]
async fn vanished_without_delete_event_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("Volume").join("Rapor.pdf");
    let mut file = describe_missing(&gone);
    file.identity = Some(FileIdentity { dev: 0, ino: 0 });
    let mut s = start(Some(file));
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Missing { id }) => assert_eq!(id, ID),
        other => panic!("expected missing, got {other:?}"),
    }
    // Reported once, not on every check.
    nothing_for(&mut s.events, Duration::from_secs(1)).await;
}

/// A record for a file that is not on disk (its volume was unplugged, say).
fn describe_missing(path: &Path) -> Watched {
    Watched {
        id: ID,
        path: path.to_path_buf(),
        seen: ec_sync::Seen {
            size: 12345,
            mtime_ns: 1,
        },
        content: ec_sync::Content::default(),
        identity: None,
    }
}

#[cfg(unix)]
#[tokio::test]
async fn unreadable_file_is_missing_once() {
    use std::os::unix::fs::PermissionsExt;
    let mut s = start(None);
    std::fs::write(&s.path, b"changed, then access lost").unwrap();
    std::fs::set_permissions(&s.path, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::File::open(&s.path).is_ok() {
        eprintln!("skipped: running with permission to read any file");
        return;
    }
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Missing { id }) => assert_eq!(id, ID),
        other => panic!("expected missing, got {other:?}"),
    }
    nothing_for(&mut s.events, Duration::from_secs(1)).await;
    // Readable again: found.
    std::fs::set_permissions(&s.path, std::fs::Permissions::from_mode(0o644)).unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Found { id, .. }) => assert_eq!(id, ID),
        other => panic!("expected found, got {other:?}"),
    }
}

#[tokio::test]
async fn an_old_remove_event_does_not_make_a_later_loss_a_delete() {
    let mut s = start(None);
    // An editor saves by writing a new file over the old one: the old one is
    // reported removed, but the file is there.
    let tmp = s.path.with_file_name("Rapor.pdf.tmp");
    std::fs::write(&tmp, b"saved").unwrap();
    std::fs::remove_file(&s.path).unwrap();
    std::fs::rename(&tmp, &s.path).unwrap();
    assert!(matches!(
        next(&mut s.events, Duration::from_secs(5)).await,
        Some(WatchEvent::Changed { .. })
    ));
    // Later the file vanishes without a delete (its volume went away, say).
    let away = s._dir.path().join("elsewhere");
    std::fs::create_dir_all(&away).unwrap();
    std::fs::rename(&s.path, away.join("Rapor.pdf")).unwrap();
    match next(&mut s.events, Duration::from_secs(5)).await {
        Some(WatchEvent::Missing { .. }) => {}
        other => panic!("expected missing, got {other:?}"),
    }
}
