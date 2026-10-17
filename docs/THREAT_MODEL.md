# Threat model

This document lists what Ecosystem Connector protects, who it protects against, and
which risks remain. It is updated whenever a feature changes what crosses the network.
Last reviewed for **phase 1** (identity, pairing, discovery, connections).

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
- **Fuzzing not yet in place.** Message decoding is size-limited and unit tested, but
  fuzz targets are planned for phase 2, together with the transfer protocol.

## Cryptography used

All primitives come from audited, widely used libraries. Nothing is implemented here.

| Purpose | Primitive | Library |
|---|---|---|
| Device key and TLS signatures | Ed25519 | `ring` via `rustls` and `rcgen` |
| Transport encryption | TLS 1.3 (AES-GCM / ChaCha20-Poly1305) over QUIC | `rustls`, `quinn` |
| Device id, pairing proof and code | BLAKE3 (keyed mode for proof and code) | `blake3` |
| Invite secret | 256 bits from the OS secure random source | `rand` |
