# Threat model

This document lists what ECOnnector protects, who it protects against, and
which risks remain. It is updated whenever a feature changes what crosses the network.
Last reviewed for **phase 3b** (fling, arrival cards, pairing links, receive folder).

## Assets

| Asset | Where it lives |
|---|---|
| Device private key (Ed25519) | `identity.key` in the data folder, mode `0600` on Unix |
| List of paired devices | `trusted-devices.json` in the data folder |
| Content sent between devices (files, clipboard, links; later phases) | In transit on the LAN, then on the receiving device |

## Trust boundaries and assumptions

- **Trusted:** the operating system account that runs the app on each device, and the
  user who confirms pairing.
- **Untrusted:** everything else on the network, including other devices on the same
  Wi-Fi, the router, and anything that answers mDNS queries.
- The app is not designed to protect against malware running as the same user on a
  device. Such malware can read the key file and every file the user can read.

## Attackers considered

| Attacker | Can do | Mitigation |
|---|---|---|
| **Passive eavesdropper** on the LAN | Read all traffic | Every connection is QUIC with TLS 1.3. Nothing is sent in clear text except mDNS announcements. |
| **Active man in the middle** (rogue router, ARP spoofing) | Intercept and modify traffic, impersonate a device | Certificates are pinned by `DeviceId` (BLAKE3 of the certificate). The initiator learns the responder's id from the QR code, so a MITM cannot complete the handshake. TLS still checks the handshake signature, proving key possession. |
| **Unpaired device** on the LAN | Connect to a device's node | The node's TLS verifier refuses any client certificate that is not in the trust store, during the handshake and before any application data. Tested in `unpaired_client_is_refused`. |
| **Fake mDNS announcements** | Advertise a known device id at an attacker's address | Announcements only point to addresses. Connecting still requires the pinned certificate, so the attacker's handshake fails (`connecting_to_an_impostor_fails`). Worst case: a device appears offline. |
| **Someone who photographs the QR code** | Try to pair before the real device | They would pair as *their* device: both screens show a verification code and the user must confirm on each side. A device that is not in front of the user never shows the matching code. The invite also works only once and expires after 2 minutes. |
| **Invite replay or relay** | Reuse a captured proof on another connection | The pairing proof and the verification code include TLS keying material exported from that session (channel binding), so they are useless on any other connection. |
| **Brute forcing the invite secret** | Guess the 256-bit secret | Not feasible, and the first wrong proof closes the pairing window. |
| **Malicious paired device or peer** | Send malformed or huge messages, hostile device names | Messages are limited to 64 KiB before allocation, decoding rejects trailing bytes, device names are stripped of control and bidi characters and cut to 64 characters (prevents terminal or UI spoofing). |
| **Malicious sender: file names** | Write outside the receive folder (`../`, absolute paths, symlinked folders), overwrite files, disguise extensions, use names Windows treats as devices | Every received path is a `RelPath`: 1–32 components, no `.`/`..`, separators, drive colons, control or bidi characters, trailing dots/spaces or reserved Windows names. Folders are created one by one and refused if they are links. Files are never overwritten (` (1)` is appended). Tested in `path_traversal_in_an_offer_is_refused` and fuzzed (`rel_path`). |
| **Malicious sender: data** | Corrupt files, send more data than announced, exhaust memory | Each chunk is checked against its BLAKE3 hash before it is written; a stream must hold exactly the chunk's length. At most 256 MiB of chunk data is held in memory and 8 transfers run at once. Offers are limited to 100 000 files and 8192 chunks per file. Tested in `corrupted_chunk_is_rejected_and_can_be_resent`. |
| **Address spoofing** | Answer at a paired device's remembered address | The remembered address is only a hint for where to dial; the handshake still requires the pinned certificate, so an impostor at that address fails like any other. |
| **Hijacking another device's transfer** | Send chunks into a transfer started by a different device | Chunk streams are matched to a transfer by id **and** by the authenticated device of the connection; chunks from another device are dropped. |

## Apps: fling, arrival cards and pairing links (phase 3b)

| Concern | Design |
|---|---|
| **A paired device makes this one open things** | Nothing received is opened automatically. Files and pages arrive as a card; the user chooses to open them, show them in the folder or save them elsewhere. Flung web pages must be `http`/`https`, at most 4096 bytes, with no whitespace or control characters (`validate_url`), and are opened only when the user taps "Open in browser". |
| **Screen access (Android accessibility service)** | Optional and off until the user turns it on in the system settings. It records only which app is in front. Only when the user touches the fling handle, it presses Copy in the selection menu the app shows (if text is selected; when an app ignores that press, a single tap is sent to the Copy item, the only gesture the service ever performs; some apps hide that menu during the swipe, so this happens as the finger lands), and when the swipe completes it reads the link of an open link menu or the browser's address bar, or takes a screenshot; the result goes only to the device the user drops it on. Copy is looked for only in small menu windows, never in the app's own window, so a Copy button on a page is not pressed. To read the copied text, a one-pixel window takes focus for a moment (Android lets only the app with focus read the clipboard); the clipboard is read only right after pressing Copy, never to see what the user copied before. Screenshots are kept in the app's cache. |
| **Drawing over other apps (Android)** | Used for the edge handle and the fling layer. Taps and vertical swipes on the handle do nothing, so brushing the edge does not open the layer. The layer covers the screen only while flinging and closes on a tap outside the card; arrival cards use a window the size of the cards at the top of the screen. |
| **Share target (Android)** | Any app can share to ECOnnector; that is the purpose of a share target. What is shared only opens as a card on this device and goes nowhere until the user drags it onto a device. Shared files are copied into the app's cache under a sanitized name; text is limited to 64K characters, and the receiver strips control characters other than line breaks and tabs before showing it. Received text is never run or opened, only offered to copy. |
| **Four-finger gesture** | Not offered. Android reports multi-finger gestures only in touch exploration mode (the TalkBack mode), which changes how every touch and many apps behave. With a full-screen passthrough region normal touches work again, but then no gesture is reported at all (tested on a Galaxy Tab S10+). The swipe in from the edge handle stays the only trigger. |
| **`ecpair:` links** | Opening a pairing link (for example by scanning the QR code with the system camera) only starts the join flow with the invite filled in. The user still compares the verification code on both screens and confirms, exactly as with the in-app scanner. |
| **Receive folder** | The chosen folder is stored in `settings.json` next to the trust store. On macOS the sandbox allows writing there through a security-scoped bookmark for that one folder; going back to the default releases it. A chosen folder that has disappeared falls back to the default instead of failing. Received paths stay confined to the folder as before. |
| **Opening received files on Android** | Files are shared with the chosen app through a `FileProvider` limited to `Download/ECOnnector`, with a read grant for that one file. |

## Known risks and limitations (phase 1)

- **Key stored as a file.** The private key is protected by file permissions only. Moving
  it into the macOS Keychain, Windows DPAPI and the Android Keystore is planned.
- **mDNS reveals presence.** Anyone on the LAN can see that a device runs Ecosystem
  Connector, with its name and id. An option to hide the name is planned.
- **Pairing window denial of service.** A device on the LAN that connects first with an
  invalid proof closes the pairing window. The user has to start pairing again. This
  is the trade-off for a single-use invite.
- **No revocation broadcast.** `ec forget` removes a device locally only. The other
  device still lists it until it is removed there too. A connection from the forgotten
  device is refused either way.
- **IPv4 only.** The node listens on IPv4. IPv6 support needs scoped link-local
  addresses and is planned.
- **Remembered addresses are stored.** `trusted-devices.json` now holds the last known
  addresses of paired devices (local network addresses, not secrets).
- **Phones and tablets behind a VPN** cannot accept connections from the Wi-Fi; the app
  shows a notice and relies on connections it opens itself.
- **No disk quota.** A paired device can send as much as the receiving disk holds.
  Paired devices are trusted with this; a per-transfer confirmation for large offers is
  planned for the apps.
- **Partial files are kept.** Unfinished transfers stay in
  `.ecosystem-connector-partial/` so they can resume. Automatic cleanup of old partials
  is planned.

## Testing

- Parsers that read untrusted input are fuzzed with `cargo-fuzz` (`fuzz/`): pairing
  invites, transfer control messages and received paths. CI runs every target on each
  change to the core and for 15 minutes each week. Fuzzing has already found one bug: a
  panic on multi-byte file names, fixed with a regression test.
- End-to-end tests run real QUIC connections and include hostile peers (wrong secrets,
  impostor certificates, unpaired devices, corrupted chunks, path traversal).

## Cryptography used

All primitives come from audited, widely used libraries. Nothing is implemented here.

| Purpose | Primitive | Library |
|---|---|---|
| Device key and TLS signatures | Ed25519 | `ring` via `rustls` and `rcgen` |
| Transport encryption | TLS 1.3 (AES-GCM / ChaCha20-Poly1305) over QUIC | `rustls`, `quinn` |
| Device id, pairing proof and code | BLAKE3 (keyed mode for proof and code) | `blake3` |
| File chunk integrity | BLAKE3 | `blake3` |
| Invite secret | 256 bits from the OS secure random source | `rand` |
