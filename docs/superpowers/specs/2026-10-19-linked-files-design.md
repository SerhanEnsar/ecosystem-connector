# Linked files (Phase 4, 0.4.0): design

Status: approved in discussion on 2026-10-19, awaiting review of this document.

## Goal

One file, one current version on every linked device, without copies piling up
(`Report (1).pdf`, `Report (2).pdf`, …) and without wearing the device out. Any file type.

## Scope

In 0.4.0:

- Linking **single files** of any type.
- **Android and macOS** apps. The Rust core also runs on Windows; the Windows UI follows
  in 0.4.x.
- A log of changes per file, conflict handling, move tracking and a control screen.

Not in 0.4.0:

- Linking folders (0.5.0, on the same journal).
- Keeping old versions of a file. Only a conflicting version is kept, until the user
  chooses.
- Content-defined chunking. Considered and deferred (see [Delta](#delta)).

## Decisions

| Topic | Decision |
|---|---|
| Change detection | Size and modification time on the device, a hash only when one of them changed. Modification times are never compared between devices. |
| Ordering between devices | A version vector per file (each device numbers its own changes), stored in the journal. |
| Location on the receiving device | `<receive folder>/Linked/<YYYY-MM-DD>/<name>`, where the date is the day the link first arrived. It never changes afterwards. Two different linked files with the same name on the same day: the second becomes `name (2).ext`. |
| Location on the linking device | macOS: the file stays where it is. Android: the shared file is copied into `Linked/<date>/` and that copy is the linked file, because Android gives no lasting access to a shared file's original. |
| Deletion | The deleting device leaves the link. Its copy is gone; the other devices keep theirs and keep syncing. A notice is shown on the deleting device and on the others. |
| Conflict | A conflict card (in the style of the arrival cards): both versions with device, time and size, and *Keep Mac's / Keep Tablet's / Open both*. Sync of that file waits until the user chooses. |
| History | No old versions on normal updates. A conflicting version is kept until the user chooses, then moved to the system Trash (deleted on Android, which has no general Trash). Pending conflict versions are listed with their size on the control screen and can be deleted there. |
| How to link | A *Send / Send linked* choice in Fling, Share → ECOnnector and the device list (default *Send*, last choice remembered), and *Link a file* on the control screen. |
| Rename and move | Local to each device. Only content is synced. The control screen shows a different name on another device ("on Mac: Report-final.pdf"). |
| When in doubt | Never break a link because a file cannot be found. It becomes *Missing* instead (see [Moves](#moves-renames-and-missing-files)). |

## Components

| Unit | Responsibility | Depends on |
|---|---|---|
| `ec-sync::journal` | Records of linked files and the change log, on disk. No networking. | `ec-core` |
| `ec-sync::watch` | File events, the cheap size and modification-time check, hashing, move detection. | `notify`, `journal` |
| `ec-sync::service` | Sync messages on stream kind 4, decisions, starting transfers. | `journal`, `watch`, `ec-net`, `ec-transfer` |
| `ec-transfer` (changed) | Chunks reused by hash instead of by index; a *replace this file* target. | — |
| `ec-runtime` (changed) | Starts the service, forwards its events, adds the link and resolve calls. | all |
| `ec-ffi`, `packages/ec_ui`, apps | API, the send choice, the conflict card, the control screen, platform watching and file access. | `ec-runtime` |

`journal` and `watch` know nothing about the network and are tested on their own.

## Journal

Kept under `<home>/linked/`. One record per linked file:

```
LinkedFile {
  id: [u8; 16]                  // random, the same on every device
  name: String                  // name on this device
  local: LocalRef               // path, plus a macOS bookmark or an Android document id
  identity: Option<FileIdentity> // device and inode, where the platform has them
  peers: Vec<DeviceId>          // devices this file is linked with
  version: VersionVector        // { device id -> counter }
  content: { size, chunk_size, hashes }
  seen: { size, mtime }         // what this device last saw on disk
  state: Current | Updating | Conflict { theirs } | Missing
}
```

Each file also has an append-only log: who changed it, on top of which version, when,
size, and moves ("14:20 ~/Documents/Report.pdf → ~/Desktop/Work/Report.pdf"). No file
contents. The log is trimmed to its last 200 entries.

Records are written to a temporary file and renamed into place (`ec-core::fsutil`), so a
crash never leaves a half-written journal.

**Decision rule.** For an incoming version `V` and the local version `L`:

- `V ≥ L` for every device: an update; fetch it.
- `L ≥ V` for every device: nothing to do (the other side is behind and will be sent
  `L`).
- otherwise: a conflict.

## Protocol

New stream kind `4`, sync control: bidirectional, framed like the other control streams
(postcard, at most 64 KiB per frame). Contents travel as ordinary transfers on kinds 1
and 2, so resume, several paths and cable boost work unchanged.

| Message | When | Carries |
|---|---|---|
| `Link { id, name, version, size, chunk_size, hashes }` | *Send linked* | A new linked file. Its content follows as a transfer. |
| `Changed { id, version, size, chunk_size, hashes }` | A local change | The new version. |
| `Summary { entries: [(id, version)] }` | Every time two devices connect | The versions of all files they share. |
| `Unlinked { id }` | A file was deleted on a device | That device left the link. |
| `Resolved { id, version }` | The user chose in a conflict card | The winning version; the other side applies it. |

Hash lists longer than one frame are split as the transfer offer already does (at most
1024 hashes per message).

`Hello` gains a capability field. *Send linked* is offered only for devices that
announce sync. A 0.3 device stops kind 4 streams as unknown, which is harmless.

## Sync flow

1. A file changes on the Mac. The watcher waits until there have been no writes for 2 s,
   then compares size and modification time with `seen`. If neither changed, nothing
   happens. Otherwise it hashes the file.
2. If the hashes changed, the Mac increments its counter, writes the log entry and sends
   `Changed` to the file's peers.
3. The tablet applies the decision rule. For an update it starts a transfer whose target
   is *replace the linked file*. Chunks whose hash it already has locally (at any index)
   are not requested.
4. The new content is completed and verified in the partial folder, then renamed over
   the linked file in one step. Never ` (1)`.
5. Just before the rename, the tablet checks the file's size and modification time
   against `seen`. If the user changed it meanwhile, it does not replace it: that is a
   conflict.
6. After writing, the tablet records the new size and modification time in `seen`, so
   its watcher does not take its own write for a local change.

While devices are apart, changes are only logged. When they connect they exchange
`Summary` and the side that is behind catches up. Changes on both sides meanwhile are a
conflict.

**Resolving a conflict.** The two version vectors are merged (the larger counter for each
device) and the counter of the device where the user chose is incremented. The chosen
content gets that version and is sent as `Resolved`. The other version is moved to the
system Trash on macOS; Android has no general Trash for these files, so it is deleted
there. *Open both* opens the two versions and keeps the card.

## Moves, renames and missing files

A path is not an identity. Each linked file also has an identity that survives a move,
and every move is logged.

- **macOS.** Each linked file has a security-scoped bookmark. It resolves the file after
  a move or rename on the same volume and reports itself stale; the new path is recorded
  and the bookmark renewed. FSEvents rename events trigger the resolution at once. The
  device and inode are a second check.
- **Android.** Linked files live in `Linked/`, so the search is small. When a file is not
  where it was, the `Linked/` tree is scanned for a file of the same size and
  modification time, and its first chunk hash is checked. A match is a move.

| What happened | How it is told | What is done |
|---|---|---|
| Moved or renamed | The bookmark or identity finds a new path. | The new path is recorded and syncing continues. |
| Deleted | The file went to the Trash, or the identity is nowhere and the file system reported a delete. | The device leaves the link and notices are shown. |
| Missing | Neither is certain: another volume, lost permission, an ejected disk. | The link stays. The file is marked *Missing* with a notice: "Report.pdf cannot be found · Show where it is · Leave link". The other devices keep syncing. When the file is back, or the user points to it, tracking resumes. |

## Watching on each platform

- **macOS.** `notify` (FSEvents) on the folder of each linked file, filtered by name.
  Access across restarts through the bookmark, as `FolderAccess.swift` does for the
  receive folder.
  *To verify first:* whether a file arriving through the Share extension can give the app
  lasting access. If not, the first link asks once for access with a panel that has the
  file already selected.
- **Android.** SAF folders give no reliable change notifications, so linked files are
  checked by size and modification time: every 5 s while the app is open, every 30 s
  while the receive service runs. `FileObserver` is used as well where the folder is a
  real path. Nothing is checked while the device sleeps; `Summary` catches up on wake or
  reconnect.

## Delta

Chunks stay fixed at 4 MiB, but the receiver reuses any local chunk with a matching hash,
not only one at the same index. This handles files changed in place or appended to
(media, databases, logs). It does not help with insertions in the middle, and no method
helps much with Office, Keynote and Pages files, which are zip archives largely rewritten
on every save; on a local network a few MB resent costs under a second.

Content-defined chunking (FastCDC) is deferred until measurements with `bench/` on real
files show a need. The manifest gains a version field so variable chunks can be added
later.

## User interface (`packages/ec_ui`)

1. *Send / Send linked* in the Fling card, the Share card and the device list.
2. Arrival card for a linked file: a 🔗 badge and "Linked, kept up to date". Later
   updates do not open a card again; they only update the text of an existing
   notification.
3. Conflict card, as above. In the background a notification comes first and opens it.
4. Settings → Linked files: each file with its devices and state (current, updating with
   progress, conflict with the waiting size, missing), the log, a different name on
   another device, and *Resolve*, *Delete waiting version*, *Show where it is*,
   *Leave link*, *Link a file*.
5. Notifications only for a left link, a conflict, a missing file and lost permission.
   Not for each successful update.

## Errors

| Case | Behaviour |
|---|---|
| Transfer interrupted | Resumes as today. The old file stays in place until the new one is verified. |
| Disk full | The update stops before touching the old file, with a notice. Retried when there is space. |
| File being written by another app | The 2 s quiet period. If the file changes while it is hashed, the result is dropped and hashing repeats. |
| Unreadable journal | Moved aside to `journal.broken`. Linked files are rescanned from disk and recovered from the peers' `Summary`. At worst this shows a conflict card; nothing is deleted. |
| Message about a file the device is not linked with, or from an unpaired device | Rejected and logged. |
| A device is unpaired | It is removed from every file's peers. Files stay where they are. |
| Peer on 0.3 | No *Send linked* for it; kind 4 is stopped by the peer. |

Additions to `THREAT_MODEL.md`: a paired device can only change files linked with it;
names go through the `RelPath` rules; a malicious peer can at most offer new versions of
files linked with it, which the user sees in the log.

## Testing

1. `journal`: version vector comparisons (after, before, concurrent), trimming, recovery
   from a broken file.
2. `watch`, in a real temporary folder: writes, moves, renames, deletes, and a counter
   test that a changed modification time with unchanged contents hashes once and sends
   nothing, and that an unchanged size and time hashes nothing.
3. Runtime tests with two and three devices (`crates/ec-runtime/tests`, without mDNS):
   - link, change, only changed chunks are sent;
   - disconnect, change on both sides, reconnect: a conflict; resolve: both agree;
   - a local change during an incoming update is not overwritten;
   - move: syncing continues; delete: the link is left and the event is emitted;
   - a 0.3-style peer without the capability.
4. Fuzzing: a `sync_message` target next to `control_message`.
5. Widget tests in `ec_ui`: conflict card, Linked files screen, badge (as
   `arrival_card_test`).
6. On devices, by hand: Mac ↔ tablet with Keynote and PDF edits, a conflict made in
   airplane mode, a move in Finder.

## Documentation to update with the code

`docs/ARCHITECTURE.md` (Linked files section, no longer *planned*), `docs/PROTOCOL.md`
(stream kind 4, `Hello` capability), `docs/THREAT_MODEL.md`, `README.md` and
`README.tr.md` (feature table and roadmap).
