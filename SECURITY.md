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

- No cloud service: traffic stays on the local network between paired devices.
- Pairing uses a QR code plus a short verification code shown on both devices.
- All connections use mutual TLS 1.3 over QUIC with pinned device keys. Connections from
  unpaired devices are refused.
- Incoming file names are sanitised; a peer can never write outside the receive folder.
- Network messages have size limits and the parsers are fuzz tested.
- Dependencies are checked against the RustSec advisory database in CI.

The full threat model will be documented in `docs/THREAT_MODEL.md`.
