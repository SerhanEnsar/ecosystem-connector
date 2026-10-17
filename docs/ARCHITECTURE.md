# Architecture

This document describes how Ecosystem Connector is put together. It is updated as each
phase lands; sections marked *planned* describe intent, not existing code.

## Goals

1. **Fast.** Wi-Fi transfers should approach the link's real throughput, and a USB cable
   should make an active transfer faster without restarting it.
2. **Secure by default.** Only devices you paired can talk to each other. All traffic is
   encrypted and authenticated. No cloud service is involved.
3. **Integrated.** It should feel like part of the operating system: a menu bar or tray
   icon on desktop, the share sheet, a Quick Settings tile and notifications on Android.
4. **Simple.** Pair once, then it just works.

Non-goals for v1: connecting over the internet, iOS support, page-level handoff inside
third-party apps.

## Components

Repository layout and module rules: [DEVELOPMENT.md](DEVELOPMENT.md).

```mermaid
flowchart TB
    subgraph app["Flutter apps (apps/android, apps/macos, apps/windows) + shared UI (packages/ec_ui)"]
        UI[Screens and tray/menu bar]
        PI[Platform integrations<br/>share sheet, QS tile, notifications,<br/>front-window document]
    end
    FFI["ec-ffi (planned)<br/>flutter_rust_bridge"]
    subgraph core["Rust core (crates/)"]
        CORE[ec-core<br/>identity, pairing, trust store]
        NET[ec-net<br/>mDNS + QUIC + mutual TLS]
        TR[ec-transfer<br/>chunks, resume, delta]
        SY[ec-sync<br/>linked files, versions]
        CB[ec-clipboard]
        HO[ec-handoff]
    end
    UI --> FFI
    PI --> FFI
    FFI --> CORE & TR & SY & CB & HO
    TR & SY & CB & HO --> NET
    NET --> CORE
```

| Crate | Responsibility |
|---|---|
| `ec-core` | Long-term device key (Ed25519), device name, pairing handshake, trust store of paired devices. |
| `ec-net` | mDNS advertisement and discovery, QUIC connections (quinn) with mutual TLS 1.3 using pinned certificates derived from device keys, path changes. |
| `ec-transfer` | Splits files into chunks, sends them over parallel streams, verifies BLAKE3 hashes, resumes from the missing chunks, sends only changed chunks of edited files. |
| `ec-sync` | Linked files: a stable file ID shared by all devices, version vectors, file watching, history of previous versions, conflict copies. |
| `ec-runtime` | A running device: identity, trust store, node, presence, remembered addresses, one connection per paired device, pairing, sending and receiving. Used by the apps and tested on its own. |
| `ec-ffi` | flutter_rust_bridge API over `ec-runtime` for the Flutter apps (crate `ec_ffi`). |
| `ec-clipboard` | Clipboard and link messages. |
| `ec-cli` | The `ec` command line tool: pairing, discovery and connection checks without the app. |
| `ec-handoff` | Broadcasts the "active item" (a file, or a URL with scroll position) so another device can offer to continue it. |

## Key flows

### Pairing

Implemented in phase 1. The exact messages are in [PROTOCOL.md](PROTOCOL.md).

```mermaid
sequenceDiagram
    participant M as Mac
    participant A as Android
    M->>M: Show QR code (device id + one-time secret + addresses)
    A->>M: Scan QR, connect
    A->>M: TLS handshake (Mac pinned by id), then proof of the secret
    Note over M,A: Both show the same 6-digit code
    M->>M: User confirms
    A->>A: User confirms
    M->>M: Store Android's device id
    A->>A: Store Mac's device id
```

### Fast transfer and cable boost

Implemented in phase 2 (`ec-transfer`). Files are split into chunks with BLAKE3 hashes
and sent over parallel QUIC streams. All needed chunks sit in one queue, and every
network path to the receiver (one QUIC connection each) runs workers that take from it.
When a new network interface appears (USB tethering on Android, a Thunderbolt or USB-C
bridge between computers), the sender looks the receiver up again and opens a
connection over the new address. That path starts taking chunks immediately, and
because a faster path acknowledges sooner, it takes more of them. Nothing restarts.

QUIC connection migration was considered and not used: QUIC only lets the client move
its own address, while a cable gives the receiver a new address too. A second
connection works in every case and also allows using Wi-Fi and the cable together.

If the connection carrying the control stream fails, the sender starts a new attempt;
the receiver hashes its partial files and asks only for what is missing. Details:
[PROTOCOL.md](PROTOCOL.md#file-transfer-kinds-1-and-2).

### Linked files (planned)

A linked file has the same ID on every device. When it changes on disk, the watcher waits
for writes to settle, hashes the chunks, and sends only the changed ones to the peers.
Each device shows whether its copy is current, updating or in conflict. If two devices
edit the same version, both are kept and the user picks one.

## Platform constraints

- **macOS has no Wi-Fi Direct**, so Mac ↔ Android without a router needs a phone hotspot
  or a cable.
- **Android 10+ blocks background clipboard reads.** Desktop → phone clipboard is
  automatic; phone → desktop uses a notification action, the Quick Settings tile or the
  share sheet.
- **Handoff** can only see what the OS exposes: the front window's document on macOS
  (Accessibility API), the URL and scroll position through the browser extension.
