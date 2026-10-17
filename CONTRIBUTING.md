# Contributing to Ecosystem Connector

Thanks for your interest in the project. This guide explains how to set up a development
environment and how changes get merged.

## Ground rules

- Be respectful. The [Code of Conduct](CODE_OF_CONDUCT.md) applies everywhere in the project.
- Report security problems privately as described in [SECURITY.md](SECURITY.md), never in
  a public issue.
- For anything larger than a small fix, open an issue first so we can agree on the approach.

## Development setup

You need:

- [Rust](https://rustup.rs), stable toolchain (the repo pins it in `rust-toolchain.toml`)
- [Flutter](https://docs.flutter.dev/get-started/install), stable channel
- For Android: Android SDK and NDK (Android Studio installs both)

```sh
cargo build
cargo test
cd app && flutter test
```

## Before you open a pull request

Run the same checks CI runs:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd app && flutter analyze && flutter test
```

Please also:

- Add tests for new behaviour and for bug fixes.
- Keep `unsafe` out of the Rust crates. The workspace forbids it; if you truly need it,
  explain why in the issue first.
- Document public items. Code that parses data from the network needs a comment on the
  limits it enforces (sizes, counts, path rules).
- Update `docs/` when you change a protocol, a flow or the architecture.

## Commit messages

Use short, imperative subject lines that say what the change does, for example
`Add resumable chunk requests` or `Fix path check for nested folders`. Explain the reason
in the body when it is not obvious.

## Where to start

Issues labelled `good first issue` are a good entry point. The roadmap in the
[README](README.md#roadmap) shows which area is being worked on.

## License

By contributing, you agree that your contributions are licensed under the
[Apache License 2.0](LICENSE), the same license as the project.
