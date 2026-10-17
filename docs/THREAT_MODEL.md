# Threat model

This document lists what Ecosystem Connector protects, who it protects against, and
which risks remain. It is updated whenever a feature changes what crosses the network.
Last reviewed for **phase 2** (identity, pairing, discovery, connections, file transfer).

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
| **Hijacking another device's transfer** | Send chunks into a transfer started by a different device | Chunk streams are matched to a transfer by id **and** by the authenticated device of the connection; chunks from another device are dropped. |

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
