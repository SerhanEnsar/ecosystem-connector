# Linked files (0.4.0) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One file, one current version on every linked device (Android and macOS), with a change log, conflict cards, move tracking and a control screen.

**Architecture:** A new `ec-sync` crate in three layers: `journal` (records and log on disk), `watch` (cheap size/mtime check, hashing, move detection) and `service` (stream kind 4 messages and decisions). Content travels as ordinary `ec-transfer` transfers that carry the link id; the receiver lets `ec-sync` place the file. `ec-runtime` wires it in, `ec-ffi` exposes it, `ec_ui` shows it.

**Tech Stack:** Rust 2024 (tokio, quinn via `ec-net`, postcard, blake3, `notify`), flutter_rust_bridge, Flutter (`packages/ec_ui`, `packages/ec_app`), Kotlin (Android), Swift (macOS).

**Spec:** `docs/superpowers/specs/2026-10-19-linked-files-design.md`

## Global Constraints

- Workspace lints apply: `unsafe_code = "forbid"`, `missing_docs = "warn"`, clippy `unwrap_used`/`expect_used` warn. New crates use `[lints] workspace = true`.
- Stream kind of sync control: `4` (`ec_sync::SYNC_KIND`). Frames as other control streams: postcard, ≤ 64 KiB (`ec_net::read_msg`/`write_msg`).
- At most 1024 hashes per message (`ec_transfer::proto::MAX_HASHES_PER_MSG`).
- Quiet period before checking a changed file: 2 s.
- Android stat check: every 5 s while the app is open, every 30 s while the receive service runs. macOS: FSEvents, with a 60 s stat check as backstop.
- Log trimmed to its last 200 entries per file.
- Receive location: `<receive dir>/Linked/<YYYY-MM-DD>/<name>`, date of first arrival, never changed; same name on the same day → `name (2).ext`.
- `Hello` is not changed. Capability is detected by the peer stopping a kind 4 stream.
- No old versions are kept; a conflict version is kept until resolved, then moved to the macOS Trash or deleted on Android.
- Never break a link when a file cannot be found: state `Missing`.
- Names and locations are local to each device; only content is synced.
- Commit messages follow the repo style (imperative sentence, no prefix) and end with the session's attribution trailers.

## Review Focus

- A local edit that lands while an incoming update is being written must survive (conflict, not overwrite). Test: Task 7 `local_edit_during_update_becomes_conflict`.
- The receiver writing an update must not trigger its own watcher into sending it back (echo loop). Test: Task 4 `own_write_is_not_a_change`, Task 7 `update_is_not_sent_back`.
- An app that saves by writing a temp file and renaming it over the original (Keynote, Office, many editors) must count as a change of the same linked file, not a delete. Test: Task 4 `atomic_save_by_rename_is_a_change`.
- A file that is unreachable because its volume is gone must not break the link. Test: Task 4 `vanished_without_delete_event_is_missing`.
- A peer that sends `Changed` for a file it is not linked with is refused. Test: Task 5 `changed_from_unlinked_peer_is_refused`.

---

### Task 0: Spike — lasting access to a file shared on macOS

Throwaway. Answers one question: can the main app create a security-scoped bookmark for a file that reaches it through the Share extension (`ShareInbox.receive` in `apps/macos/macos/Runner/AppDelegate.swift`)?

- [ ] **Step 1:** On a scratch branch, in `ShareInbox.receive`, for each file URL call `url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil)` and log success or the error with `NSLog`.
- [ ] **Step 2:** Build with `tools/check.sh macos`, run the app, share a file from `~/Documents` through Finder → Share → ECOnnector. Quit, relaunch, resolve the bookmark with `.withSecurityScope`, call `startAccessingSecurityScopedResource()` and read the file's first byte.
- [ ] **Step 3:** Record the result in the spec's *Watching on each platform* section: either "bookmark works" or "first link asks for access with an `NSOpenPanel` preselected to the file". Delete the scratch branch. Commit the spec change only.

```bash
git add docs/superpowers/specs/2026-10-19-linked-files-design.md
git commit -m "Record how the Mac app keeps access to a shared linked file"
```

### Task 1: Journal — version vectors and records on disk

**Files:**
- Modify: `crates/ec-sync/Cargo.toml` (deps: `ec-core`, `serde` with derive, `postcard` with `use-std`, `rand`, `thiserror`, `tracing`; dev: `tempfile`)
- Modify: `crates/ec-sync/src/lib.rs` (modules, `Error`, `Result`, re-exports)
- Create: `crates/ec-sync/src/version.rs`, `crates/ec-sync/src/journal.rs`
- Modify: `crates/ec-core/src/fsutil.rs`, `crates/ec-core/src/lib.rs` (make `write_private`, `create_private_dir` and the existing `read_optional` reader `pub`, re-exported as `ec_core::fsutil`)
- Test: unit tests in `version.rs` and `journal.rs`

**Interfaces:**
- Produces:
  - `pub type LinkId = [u8; 16];` `pub fn new_link_id() -> LinkId`
  - `pub struct VersionVector(BTreeMap<DeviceId, u64>)` with `bump(&mut self, me: DeviceId)`, `merged(&self, other: &Self) -> Self`, `compare(&self, other: &Self) -> Order`; `pub enum Order { Equal, Before, After, Concurrent }` (`After` = self dominates).
  - `pub struct Content { pub size: u64, pub chunk_size: u32, pub hashes: Vec<[u8; 32]> }`
  - `pub struct Seen { pub size: u64, pub mtime_ns: i128 }`
  - `pub struct FileIdentity { pub dev: u64, pub ino: u64 }`
  - `pub enum State { Current, Updating, Conflict { theirs: Content, from: DeviceId, at_ms: u64 }, Missing }`
  - `pub struct LinkedFile { pub id: LinkId, pub name: String, pub path: PathBuf, pub identity: Option<FileIdentity>, pub peers: Vec<DeviceId>, pub version: VersionVector, pub content: Content, pub seen: Seen, pub state: State }`
  - `pub enum LogKind { Linked, Changed, Received, Moved { from: PathBuf, to: PathBuf }, Conflict, Resolved, Unlinked, PeerLeft(DeviceId), Missing, Found }`
  - `pub struct LogEntry { pub at_ms: u64, pub by: DeviceId, pub kind: LogKind, pub version: VersionVector, pub size: u64 }`
  - `pub struct Journal` with `open(dir: &Path) -> Result<Journal>`, `get(&self, id: &LinkId) -> Option<&LinkedFile>`, `files(&self) -> impl Iterator<Item = &LinkedFile>`, `put(&mut self, file: LinkedFile) -> Result<()>`, `remove(&mut self, id: &LinkId) -> Result<Option<LinkedFile>>`, `log(&self, id: &LinkId) -> Result<Vec<LogEntry>>`, `append(&mut self, id: &LinkId, entry: LogEntry) -> Result<()>`, `remove_peer(&mut self, peer: &DeviceId) -> Result<()>`, `recovered(&self) -> bool`.
- On-disk layout: `<dir>/journal.bin` (all records, postcard, written with `ec_core::fsutil::write_private`), `<dir>/log/<hex id>.bin` (entries, rewritten atomically, trimmed to 200).

- [ ] **Step 1: Write the failing tests** in `version.rs`:
  - `vectors_order`: `{A:1}` vs `{A:1}` → `Equal`; `{A:2}` vs `{A:1}` → `After`; `{A:1}` vs `{A:1,B:1}` → `Before`; `{A:2}` vs `{A:1,B:1}` → `Concurrent`; missing entries count as 0.
  - `merge_then_bump_dominates_both`: `a={A:2}`, `b={A:1,B:3}`; `m=a.merged(&b)` then `m.bump(A)` → `m.compare(&a)==After && m.compare(&b)==After`, `m == {A:3,B:3}`.
- [ ] **Step 2:** `cargo test -p ec-sync version` → FAIL (types missing).
- [ ] **Step 3:** Implement `version.rs`.
- [ ] **Step 4: Write the failing tests** in `journal.rs` (each in a `tempfile::tempdir()`):
  - `records_survive_reopen`: `put` a `LinkedFile`, drop, `open` again → `get` equals it.
  - `log_is_trimmed_to_200`: `append` 250 entries → `log` has 200, the first is entry #50.
  - `broken_journal_is_moved_aside`: write garbage to `journal.bin`, `open` → `Ok`, `files().count()==0`, `recovered()==true`, `journal.broken` exists with the garbage.
  - `removing_a_peer_keeps_the_file`: file with peers `[B, C]`, `remove_peer(&B)` → peers `[C]`, file still present.
- [ ] **Step 5:** `cargo test -p ec-sync journal` → FAIL.
- [ ] **Step 6:** Implement `journal.rs` and the `ec-core` visibility change.
- [ ] **Step 7:** `cargo test -p ec-sync && tools/check.sh core` → PASS.
- [ ] **Step 8: Commit** — "Keep linked files and their change log on disk".

### Task 2: Transfer — reuse chunks by hash and place linked files

**Files:**
- Modify: `crates/ec-transfer/src/proto.rs` (new `Control::Target { key: [u8; 16] }`, sender → receiver, optional, before `OfferEnd`, at most once, only with exactly one file)
- Modify: `crates/ec-transfer/src/receiver.rs` (`read_offer` accepts `Target`; `open_partials` seeds from a local file; `finalize` hands linked files to the placer)
- Modify: `crates/ec-transfer/src/sender.rs` (`SendOptions.target: Option<[u8; 16]>`, sends `Control::Target`)
- Modify: `crates/ec-transfer/src/lib.rs` (re-exports)
- Modify: `docs/PROTOCOL.md` (File transfer: `Target`, seeding)
- Test: `crates/ec-transfer/tests/transfer.rs`

**Interfaces:**
- Produces:
  - `pub trait Placer: Send + Sync { fn place(&self, peer: DeviceId, key: [u8; 16], entry: &FileEntry) -> Option<Arc<dyn Placement>>; }` — `None` declines the offer (`Rejected { reason: "not linked" }`).
  - `pub trait Placement: Send + Sync { fn seed(&self) -> Option<PathBuf>; fn commit(&self, part: &Path) -> std::io::Result<PathBuf>; }` — `commit` is called in `finalize` after `sync_all` and close, instead of `unique_destination` + rename. Its error becomes `ReceiveEvent::Interrupted`.
  - `Receiver::set_placer(&self, placer: Arc<dyn Placer>)`.
  - `ReceiveEvent::Saved` gains `linked: bool` (`true` when placed).
  - `SendOptions { target: Option<[u8; 16]>, .. }`.
- Seeding: before listing needs, the seed file is hashed on its own chunk grid with the offer's `chunk_size` (`hash_chunks`); for every needed chunk whose hash appears in the seed, copy those bytes into the partial file at the chunk's offset and mark it `Done`. The partial key includes the target key so linked updates never share partials with plain sends.

- [ ] **Step 1: Write the failing tests** in `tests/transfer.rs` (pattern of the existing two-node tests):
  - `linked_update_reuses_chunks_by_hash`: seed file = chunks `[A, B, C]` (64 KiB chunk size), new file = `[X, A, B, C]` (shifted by one chunk). Send with `target: Some(k)` and a test placer whose `seed()` is the old file. Assert `SendReport.bytes_sent == 64 * 1024` and the committed file equals the new file.
  - `placer_declines_unknown_target`: placer returns `None` → `send_paths` returns an error whose message contains `"not linked"`.
  - `target_without_placer_is_rejected`: receiver without placer, `target: Some(_)` → rejected.
  - `commit_error_interrupts`: placement whose `commit` returns `Err` → `ReceiveEvent::Interrupted` emitted, no file in the receive dir.
  - `two_files_with_target_are_rejected`: offer of two files with a target → rejected.
- [ ] **Step 2:** `cargo test -p ec-transfer` → the new tests FAIL.
- [ ] **Step 3:** Implement `Control::Target`, `SendOptions.target`, `Placer`/`Placement`, seeding, `Saved.linked`. Keep existing behaviour identical when no target is sent.
- [ ] **Step 4:** Extend `fuzz/fuzz_targets/control_message.rs` to decode `Control::Target` without panicking (no new assertion needed beyond decode).
- [ ] **Step 5:** `cargo test -p ec-transfer && tools/check.sh core` → PASS. Update `crates/ec-runtime` and `crates/ec-ffi` for the new `Saved.linked` field (map to the FFI `Saved` event as `linked: bool`) so the workspace builds.
- [ ] **Step 6: Commit** — "Let a transfer replace a linked file and reuse its chunks".

### Task 3: Sync messages

**Files:**
- Create: `crates/ec-sync/src/proto.rs`
- Create: `fuzz/fuzz_targets/sync_message.rs`; Modify: `fuzz/Cargo.toml` (`ec-sync` dep, `[[bin]] sync_message`)
- Test: unit tests in `proto.rs`

**Interfaces:**
- Produces:
  - `pub const SYNC_KIND: u8 = 4;` `pub const MAX_NAME_BYTES: usize = 255;` `pub const MAX_SUMMARY_ENTRIES: usize = 4096;`
  - `pub enum SyncMessage { Link { id, name: String, version: VersionVector, content: ContentHeader }, Changed { id, version, content: ContentHeader }, Resolved { id, version, content: ContentHeader }, Hashes { id, start: u32, hashes: Vec<[u8; 32]> }, Summary { entries: Vec<(LinkId, VersionVector)> }, Unlinked { id }, Reply { id, want: bool } }` with `pub struct ContentHeader { size: u64, chunk_size: u32 }` (hashes follow in `Hashes`, as in the transfer offer).
  - `SyncMessage::validate(self) -> Result<Self>`: name passes `ec_transfer::RelPath::new(vec![name])` and ≤ 255 bytes; `chunk_size` in 64 KiB–64 MiB; hash count bound by `FileEntry::chunk_count_for`; `Hashes` ≤ 1024; `Summary` ≤ 4096 entries; vectors ≤ 64 devices.
  - `pub async fn send_content(send: &mut SendStream, msg: SyncMessage, hashes: &[[u8; 32]]) -> Result<()>` and `pub async fn read_content(recv: &mut RecvStream, first: SyncMessage) -> Result<(SyncMessage, Vec<[u8; 32]>)>` (collect `Hashes` until the count implied by the header).

- [ ] **Step 1: Write the failing tests:** `names_are_checked` (`"../x"`, `"a/b"`, `""`, 256-byte name rejected; `"Rapor.pdf"` accepted), `bounds_are_checked` (chunk size 1 KiB rejected; 1025 hashes rejected; 4097 summary entries rejected), `roundtrip` (postcard encode/decode of each variant equals the original).
- [ ] **Step 2:** `cargo test -p ec-sync proto` → FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Write `sync_message.rs`: decode with `ec_net::decode::<SyncMessage>`, call `validate`, and if `Link` validates, assert the name has no `/` and is not `..`.
- [ ] **Step 5:** `cargo test -p ec-sync && (cd fuzz && cargo +nightly fuzz build sync_message)` → PASS.
- [ ] **Step 6: Commit** — "Define the sync messages for linked files".

### Task 4: Watcher

**Files:**
- Create: `crates/ec-sync/src/watch.rs`, `crates/ec-sync/src/identity.rs` (`FileIdentity::of(path) -> Option<FileIdentity>` from `std::os::unix::fs::MetadataExt` `dev()`/`ino()`; `None` on other platforms)
- Modify: `crates/ec-sync/Cargo.toml` (`notify = "8"`, `tokio` with `time`, `sync`, `rt`; `ec-transfer` for `hash_chunks`/`chunk_size_for`)
- Test: `crates/ec-sync/tests/watch.rs`

**Interfaces:**
- Consumes: `Journal`, `LinkedFile`, `Seen`, `Content`, `FileIdentity` (Task 1).
- Produces:
  - `pub struct WatchConfig { pub quiet: Duration /* 2 s */, pub stat_interval: Duration, pub roots: Vec<PathBuf> /* folders searched for moved files: the Linked folder */ }`
  - `pub enum WatchEvent { Changed { id: LinkId, content: Content, seen: Seen }, Moved { id: LinkId, to: PathBuf, identity: Option<FileIdentity> }, Deleted { id: LinkId }, Missing { id: LinkId }, Found { id: LinkId, at: PathBuf } }`
  - `pub struct Watcher` with `start(cfg: WatchConfig, files: Vec<Watched>, events: mpsc::UnboundedSender<WatchEvent>) -> Result<Watcher>`, `set(&self, files: Vec<Watched>)`, `set_stat_interval(&self, d: Duration)`, `expect_write(&self, id: LinkId, seen: Seen)` (records the receiver's own write so it is not reported), `hash_count(&self) -> u64` (for tests).
  - `pub struct Watched { pub id: LinkId, pub path: PathBuf, pub seen: Seen, pub content: Content, pub identity: Option<FileIdentity> }`
  - `pub fn check(file: &Watched) -> CheckResult` (pure, blocking): `Unchanged` | `Touched(Seen)` (stat changed, hashes equal) | `Changed(Content, Seen)` | `Gone`.
- Rules: a stat change waits for `quiet` without further events before `check`. `Gone` → search `roots` by identity, then by size + mtime + first chunk hash → `Moved`; else a delete event for that path seen in the last `quiet`·2, or the path is inside a Trash folder (`/.Trash/`, `/.Trashes/`) → `Deleted`; else `Missing`. A `Missing` file that reappears at its path or is found in a root → `Found`.

- [ ] **Step 1: Write the failing tests** (real temp folders, `stat_interval` 200 ms, `quiet` 100 ms):
  - `unchanged_file_is_never_hashed`: start, wait 1 s → no events, `hash_count()==0`.
  - `touch_hashes_once_and_reports_nothing`: set mtime only (`filetime` dev-dependency) → no `Changed`, `hash_count()==1`.
  - `edit_is_reported_once`: append bytes → exactly one `Changed` with the new size.
  - `own_write_is_not_a_change`: `expect_write(id, seen_after)` then write those bytes → no event.
  - `atomic_save_by_rename_is_a_change`: write `x.tmp`, rename over the linked path → one `Changed`, no `Deleted`.
  - `move_inside_root_is_followed`: rename into a subfolder of the root → `Moved { to }`.
  - `delete_is_reported`: `remove_file` → `Deleted`.
  - `vanished_without_delete_event_is_missing`: point a `Watched` at a path that never existed, with an identity not present in roots → `Missing` (no delete event was seen).
- [ ] **Step 2:** `cargo test -p ec-sync --test watch` → FAIL.
- [ ] **Step 3:** Implement `check`, then `Watcher` (one `notify::RecommendedWatcher` on each linked file's parent folder, events filtered by path; a tokio interval for stat checks; blocking work in `spawn_blocking`).
- [ ] **Step 4:** `cargo test -p ec-sync` → PASS (run 5× to catch timing flakiness: `for i in 1 2 3 4 5; do cargo test -p ec-sync --test watch || break; done`).
- [ ] **Step 5: Commit** — "Watch linked files without hashing unchanged ones".

### Task 5: Sync service

**Files:**
- Create: `crates/ec-sync/src/service.rs`, `crates/ec-sync/src/place.rs`
- Modify: `crates/ec-sync/Cargo.toml` (`ec-net`, `ec-transfer`)
- Test: unit tests in `service.rs` for decisions; network behaviour is tested in Task 7.

**Interfaces:**
- Consumes: Tasks 1–4; `ec_net::{Service, PeerConnection, read_msg, write_msg}`; `ec_transfer::{Placer, Placement, FileEntry}`.
- Produces:
  - `pub fn decide(local: &LinkedFile, incoming: &VersionVector, incoming_hashes: &[[u8;32]]) -> Decision`; `pub enum Decision { Ignore /* local is ahead or equal */, Update, Conflict, AdoptVersion /* same content, newer vector */ }`.
  - `pub enum SyncEvent { Linked { id, from: DeviceId }, Updated { id }, Conflict { id }, Resolved { id }, PeerUnlinked { id, peer: DeviceId }, LeftLink { id }, Missing { id }, Found { id }, Moved { id }, Progress { id, received: u64, total: u64 }, Error { id, message: String } }`
  - `pub struct SyncConfig { pub me: DeviceId, pub home: PathBuf /* journal dir = home/linked */, pub linked_root: PathBuf /* <receive dir>/Linked */, pub trash: Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync> /* macOS: move to Trash; Android: remove_file */, pub now_ms: Arc<dyn Fn() -> u64 + Send + Sync> }`
  - `pub trait Outbox: Send + Sync { fn send_file(&self, peer: DeviceId, path: PathBuf, key: LinkId) -> BoxFuture<Result<()>>; fn conn(&self, peer: &DeviceId) -> Option<PeerConnection>; }` (implemented by `ec-runtime` over `send_paths` with `SendOptions.target`).
  - `pub struct SyncService` with `start(cfg, outbox: Arc<dyn Outbox>, watch: WatchConfig, events: broadcast::Sender<SyncEvent>) -> Result<Arc<SyncService>>`, implements `ec_net::Service` (kind 4) and `ec_transfer::Placer`; methods `link(&self, peer: DeviceId, path: PathBuf, copy_into_linked: bool) -> Result<LinkId>`, `files(&self) -> Vec<LinkedFile>`, `log(&self, id) -> Result<Vec<LogEntry>>`, `resolve(&self, id, keep_theirs: bool) -> Result<()>`, `delete_conflict(&self, id) -> Result<()>`, `leave(&self, id) -> Result<()>`, `relocate(&self, id, path: PathBuf) -> Result<()>`, `connected(&self, peer: DeviceId)` (sends `Summary`; on stopped stream marks the peer unsupported), `supports(&self, peer: &DeviceId) -> Option<bool>`, `forget_peer(&self, peer)`, `set_stat_interval(&self, d)`.
- Placement (`place.rs`): `Decision::Update` → seed = linked path, commit = check `seen` still matches the file (else store as conflict and return the conflict path), then `rename(part, linked path)`, `expect_write`, update record. `Decision::Conflict` → commit = move part to `<home>/linked/conflicts/<hex id>`, state `Conflict`. A first `Link` → commit = create `Linked/<YYYY-MM-DD>/` (date from `now_ms`, UTC), name `name` or `name (2).ext`… via the existing `unique_destination` rule.
- Messages are only accepted for ids whose `peers` contains the connection's peer (except `Link`, which creates it), otherwise the stream is closed and the refusal logged.
- `link` on a path that is already linked adds the peer to that link (same id, `Link` sent to the new peer only) instead of creating a second link.
- A failed incoming transfer (disk full, interrupted) leaves the file as it was and emits `SyncEvent::Error`; the next `Summary` (every reconnect) or the next `Changed` retries it.

- [ ] **Step 1: Write the failing tests** in `service.rs`:
  - `decide_update_ignore_conflict_adopt`: local `{A:2}` hashes `h1`; incoming `{A:3}`/`h2` → `Update`; `{A:1}` → `Ignore`; `{A:1,B:1}`/`h2` → `Conflict`; `{A:2,B:1}`/`h1` → `AdoptVersion`.
  - `changed_from_unlinked_peer_is_refused`: service with a file linked to `B`; feed a `Changed` from `C` through the handler helper `handle_message(peer, msg)` → `Err`, journal unchanged.
  - `first_link_goes_to_dated_folder`: `place` for a new `Link` named `Rapor.pdf` with `now_ms` = 2026-10-19 → commit path ends with `Linked/2026-10-19/Rapor.pdf`; a second different id with the same name the same day → `Rapor (2).pdf`.
  - `linking_again_adds_a_peer`: `link(B, p)` then `link(C, p)` → one record, peers `[B, C]`, same id.
  - `resolve_keep_theirs_trashes_mine`: file in `Conflict`; `resolve(id, true)` → version = merged + bump(me), linked file now has their bytes, `trash` called once with a path holding the old bytes, state `Current`.
- [ ] **Step 2:** `cargo test -p ec-sync service` → FAIL.
- [ ] **Step 3:** Implement `decide`, `place.rs`, `SyncService` (message loop per stream with `read_content`, `Reply` handling, watcher events → bump + log + `Changed` to peers, `Summary` on connect: for each shared id, if mine is `After` or `Concurrent` push `Changed`).
- [ ] **Step 4:** `cargo test -p ec-sync && tools/check.sh core` → PASS.
- [ ] **Step 5: Commit** — "Sync linked files between paired devices".

### Task 6: Runtime wiring

**Files:**
- Modify: `crates/ec-runtime/Cargo.toml` (`ec-sync`), `crates/ec-runtime/src/lib.rs`

**Interfaces:**
- Consumes: Task 5.
- Produces (on `Runtime`):
  - `Config.linked_trash: Arc<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>` (default: `std::fs::remove_file`)
  - `Event::Sync(SyncEvent)`
  - `DeviceStatus.linked_files: bool` (from `SyncService::supports`, `false` while unknown)
  - `pub async fn send_linked(&self, id: DeviceId, path: PathBuf, copy_into_linked: bool) -> Result<LinkId>`
  - `pub fn linked_files(&self) -> Vec<LinkedFile>`, `pub fn linked_log(&self, id: LinkId) -> Result<Vec<LogEntry>>`, `pub fn resolve_conflict(&self, id: LinkId, keep_theirs: bool) -> Result<()>`, `pub fn delete_conflict(&self, id: LinkId) -> Result<()>`, `pub fn leave_link(&self, id: LinkId) -> Result<()>`, `pub fn relocate_linked(&self, id: LinkId, path: PathBuf) -> Result<()>`, `pub fn set_linked_check_interval(&self, d: Duration)`
  - `Error::Sync(#[from] ec_sync::Error)`
- Wiring: register `SyncService` on the router for `SYNC_KIND`, `receiver.set_placer(sync)`; call `sync.connected(peer)` in `Shared::adopt`; `forget` calls `sync.forget_peer`; `set_receive_dir` updates the linked root; `Outbox::send_file` reuses the body of `Runtime::send` with `SendOptions.target`.

- [ ] **Step 1:** Implement (no new behaviour beyond wiring; tests in Task 7).
- [ ] **Step 2:** `cargo test -p ec-runtime` (existing tests) → PASS.
- [ ] **Step 3: Commit** — "Run the sync service in every device".

### Task 7: Runtime tests with two and three devices

**Files:**
- Create: `crates/ec-runtime/tests/linked.rs` (helpers copied from `tests/runtime.rs`: two or three runtimes with `mdns: false`, paired through `host_pairing`/`join_pairing`)

- [ ] **Step 1: Write the tests** (each waits on `Event::Sync` with a 10 s timeout):
  - `link_then_edit_sends_only_changed_chunks`: Mac `send_linked` a 12 MiB file (three 4 MiB chunks); tablet gets `Linked`, file at `Linked/<today>/name`; Mac appends 1 MiB → tablet `Updated`, bytes equal; the second transfer's `SendReport.bytes_sent` ≤ 4 MiB + 1 MiB.
  - `update_is_not_sent_back`: after the update above, wait 3 s → the Mac receives no `Updated` for that id.
  - `offline_edits_on_both_sides_conflict_and_resolve`: shut down the tablet runtime, edit both files, restart (same home), wait for `Conflict` on both; tablet `resolve_conflict(id, true)` → both files equal, both states `Current`, one `trash` call on the tablet.
  - `local_edit_during_update_becomes_conflict`: tablet linked file; a test placer hook (`ec_sync` feature `test-hooks` exposing `before_commit: Option<Arc<dyn Fn()>>` on `SyncConfig`) edits the tablet's file just before commit → tablet state `Conflict`, the tablet's edit still on disk.
  - `move_keeps_syncing_and_delete_leaves_link`: tablet moves its file inside `Linked/` → `Moved`; Mac edits → tablet `Updated` at the new path; tablet deletes → Mac gets `PeerUnlinked`, the Mac file stays.
  - `three_devices_stay_current`: Mac links to tablet and phone; phone edits → Mac and tablet both `Updated`.
  - `older_peer_is_not_offered_linked`: a runtime built without the sync service (test-only `Config.sync: false`) → the other side's `devices()` shows `linked_files == false` after connecting.
- [ ] **Step 2:** `cargo test -p ec-runtime --test linked` → PASS (fix defects found in Tasks 2–6 within those tasks' files).
- [ ] **Step 3:** `tools/check.sh core` → PASS.
- [ ] **Step 4: Commit** — "Test linked files across two and three devices".

### Task 8: FFI

**Files:**
- Modify: `crates/ec-ffi/src/api/core.rs`; regenerate with `flutter_rust_bridge_codegen generate` in `packages/ec_bridge` (updates `crates/ec-ffi/src/frb_generated.rs` and `packages/ec_bridge/lib/src/rust/**`)

**Interfaces:**
- Produces:
  - `pub struct LinkedView { pub id: String /* hex */, pub name: String, pub path: String, pub peers: Vec<String>, pub state: LinkedState, pub size: u64, pub conflict: Option<ConflictView> }`
  - `pub enum LinkedState { Current, Updating { received: u64, total: u64 }, Conflict, Missing }`
  - `pub struct ConflictView { pub mine_device: String, pub mine_at_ms: u64, pub mine_size: u64, pub theirs_device: String, pub theirs_at_ms: u64, pub theirs_size: u64, pub theirs_path: String }`
  - `pub struct LinkedLogView { pub at_ms: u64, pub device: String, pub text: String }` (English text built in Rust: "changed", "received", "moved from … to …", "left the link", …; the UI localizes later)
  - `DeviceView.linked_files: bool`
  - `CoreEvent::Linked { id: String, kind: LinkedEventKind }` with `enum LinkedEventKind { Arrived, Updated, Conflict, Resolved, PeerLeft { device: String }, Left, Missing, Found, Moved, Error { message: String } }`
  - On `EcCore`: `send_linked(device: String, path: String, copy_into_linked: bool) -> Result<String>`, `linked_files() -> Vec<LinkedView>`, `linked_log(id: String) -> Result<Vec<LinkedLogView>>`, `resolve_conflict(id: String, keep_theirs: bool)`, `delete_conflict(id: String)`, `leave_link(id: String)`, `relocate_linked(id: String, path: String)`, `set_linked_check_interval(seconds: u32)`.
- [ ] **Step 1:** Implement and regenerate.
- [ ] **Step 2:** `tools/check.sh core && tools/check.sh macos` → PASS (Dart analysis passes with the regenerated bindings).
- [ ] **Step 3: Commit** — "Expose linked files to the apps".

### Task 9: Shared UI

**Files:**
- Modify: `packages/ec_ui/lib/src/controller.dart`, `model.dart`, `testing.dart`, `fling/fling_overlay.dart`, `widgets/share_bar.dart`, `screens/send_sheet.dart`, `arrival/arrival_card.dart`, `screens/settings_screen.dart`
- Create: `packages/ec_ui/lib/src/linked/conflict_card.dart`, `packages/ec_ui/lib/src/linked/linked_files_screen.dart`
- Test: `packages/ec_ui/test/conflict_card_test.dart`, `linked_files_screen_test.dart`; extend `arrival_card_test.dart`, `fling_overlay_test.dart`

**Interfaces:**
- Consumes: Task 8 views (mirrored as Dart models).
- Produces on `EcController`: `bool get sendLinked; set sendLinked(bool)` (remembered), `Stream<SendStatus> sendLinkedFile(EcDevice device, String path)`, `List<EcLinkedFile> get linkedFiles`, `Future<List<EcLinkedLog>> linkedLog(String id)`, `Future<void> resolveConflict(String id, {required bool keepTheirs})`, `Future<void> deleteConflict(String id)`, `Future<void> leaveLink(String id)`, `Future<void> locateLinked(String id)` (platform picker, then relocate), `Future<void> linkFile()`, `EcLinkedConflict? get pendingConflict`; `EcDevice.linkedFiles`.
- Copy (English, as the rest of the UI): "Send" / "Send linked"; badge "Linked, kept up to date"; conflict card title "<name> changed on two devices", buttons "Keep <device>'s", "Keep <device>'s", "Open both"; screen title "Linked files"; states "Up to date", "Updating <n>%", "Conflict (<size> waiting)", "Missing"; actions "Resolve", "Delete waiting version", "Show where it is", "Leave link", "Link a file"; another name: "on <device>: <name>".
- [ ] **Step 1: Write the failing widget tests:**
  - `conflict_card_test`: shows both device names, times and sizes; tapping "Keep Tablet's" calls `resolveConflict(id, keepTheirs: true)`; "Open both" calls `open` twice and keeps the card.
  - `linked_files_screen_test`: three files in states Current/Updating 40%/Conflict 1.3 MB render "Up to date", "Updating 40%", "Conflict (1.3 MB waiting)"; "Delete waiting version" calls `deleteConflict`; "Leave link" asks for confirmation and then calls `leaveLink`.
  - `arrival_card_test`: a linked arrival shows the badge text.
  - `fling_overlay_test`: "Send linked" appears only for a device with `linkedFiles == true` and only for a single file; choosing it calls `sendLinkedFile`.
- [ ] **Step 2:** `cd packages/ec_ui && flutter test` → new tests FAIL.
- [ ] **Step 3:** Implement the widgets (conflict card reuses the arrival card's drop-and-spread container and theme tokens; the screen is reached from Settings).
- [ ] **Step 4:** `flutter test` → PASS.
- [ ] **Step 5: Commit** — "Show linked files, conflicts and the send choice".

### Task 10: App bridge and platforms

**Files:**
- Modify: `packages/ec_app/lib/src/bridge_controller.dart`, `hooks.dart`, `notifier.dart`
- Modify (Android): `apps/android/android/app/src/main/kotlin/io/github/serhanensar/ec_android/ShareActivity.kt`, `ReceiveService.kt`, `MainActivity.kt`, `Files.kt`
- Modify (macOS): `apps/macos/macos/Runner/AppDelegate.swift`, `MainFlutterWindow.swift`; Create: `apps/macos/macos/Runner/LinkedAccess.swift`
- Modify: `apps/android/lib/main.dart`, `apps/macos/lib/main.dart`

**Interfaces:**
- Hooks (Dart → platform): `Future<bool> trash(String path)` (macOS `NSWorkspace.shared.recycle`, Android: `File.delete`), `Future<String?> bookmarkLinked(String id, String path)` and `Future<Map<String,String>> restoreLinked()` (macOS only: `LinkedAccess` keeps one bookmark per link id in `UserDefaults` key `linkedBookmarks`, resolves on start, reports `[id: newPath]` for stale ones), `Future<String?> pickLinkedLocation(String name)`.
- Android: a file shared with *Send linked* is passed with `copy_into_linked: true`. `set_linked_check_interval(5)` on app resume, `30` when the app goes to the background while `ReceiveService` runs.
- macOS: after `send_linked` or an arrival with `linked`, call `bookmarkLinked`; on start call `restoreLinked` and `relocate_linked` for each stale entry; `set_linked_check_interval(60)`; the Trash closure in `Config.linked_trash` calls `trash` through the hook.
- Notifications (`notifier.dart`): only for `Conflict`, `PeerLeft`, `Left`, `Missing` and lost access, with the copy from Task 9. A linked update replaces the text of the existing notification ("<name> updated") instead of adding one.

- [ ] **Step 1:** Implement the bridge methods and platform code.
- [ ] **Step 2:** `tools/check.sh android && tools/check.sh macos` → PASS.
- [ ] **Step 3:** Manual check on devices (Mac ↔ tablet): link a PDF from Finder (Share → ECOnnector → Send linked); edit it in Preview → tablet updates without a second card; edit the tablet copy → Mac updates; airplane mode on the tablet, edit both, reconnect → conflict cards on both, resolve on one → both equal, the other version in the Mac Trash; move the Mac file in Finder → still syncs, control screen log shows the move; delete it on the tablet → Mac notice, Mac file stays. Record results in the PR description.
- [ ] **Step 4: Commit** — "Linked files on Android and the Mac".

### Task 11: Documentation

**Files:**
- Modify: `docs/ARCHITECTURE.md` (Linked files section, no longer *planned*; `ec-sync` row), `docs/PROTOCOL.md` (stream kind 4 table row, Sync section with the message table and capability detection), `docs/THREAT_MODEL.md` (a *Malicious peer: linked files* row), `README.md` and `README.tr.md` (Linked files status "Android, macOS, pre-release"; roadmap Phase 4 checked).

- [ ] **Step 1:** Update the documents to match what was built.
- [ ] **Step 2:** `tools/check.sh core` (doc links checked if the script does) → PASS.
- [ ] **Step 3: Commit** — "Document linked files".
