# Security Policy

Ecosystem Connector moves files and clipboard contents between devices, so security issues
are taken seriously.

## Reporting a vulnerability

**Do not open a public issue.** Report it privately through GitHub:

1. Go to the [Security tab](https://github.com/SerhanEnsar/ecosystem-connector/security).
2. Click **Report a vulnerability**.

Please include the affected version or commit, steps to reproduce, and the impact you
expect. You will get an acknowledgement within 7 days. Once a fix is ready, it is released
and the advisory is published, with credit to you if you want it.

## Supported versions

The project is in early development. Only the latest commit on `main` receives fixes.

## Security design

In place today:

- No cloud service: traffic stays on the local network between paired devices.
- Pairing uses a single-use QR code invite plus a 6-digit verification code that the
  user confirms on both devices.
- All connections use mutual TLS 1.3 over QUIC with pinned device keys. Handshakes from
  unpaired devices are refused.
- Network messages are size-limited before any allocation; device names from other
  devices are sanitised.
- Received file names are validated so a peer can never write outside the receive
  folder or overwrite a file; every chunk is verified with BLAKE3 before it is written.
- Every parser of untrusted input is fuzzed in CI.
- Dependencies are checked against the RustSec advisory database in CI.

Planned:

- Private keys moved into the OS keychain or keystore.
- Confirmation for large incoming transfers and cleanup of old partial files.

The full analysis is in [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md).
