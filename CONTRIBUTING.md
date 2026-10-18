# Contributing to ECOnnector

Thanks for your interest in the project. This guide explains how to set up a development
environment and how changes get merged.

## Ground rules

- Be respectful. The [Code of Conduct](CODE_OF_CONDUCT.md) applies everywhere in the project.
- Report security problems privately as described in [SECURITY.md](SECURITY.md), never in
  a public issue.
- For anything larger than a small fix, open an issue first so we can agree on the approach.

## Development setup

Each area has its own environment; you only need the one you work on. The full guide,
including the module rules, is in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

| Area | Folder | Needs |
|---|---|---|
| Core | `crates/` | Rust stable |
| Shared UI | `packages/ec_ui/` | Flutter stable |
| Android | `apps/android/` | Flutter, Android Studio |
| macOS | `apps/macos/` | Flutter, Xcode |
| Windows | `apps/windows/` | Flutter, Visual Studio 2022 (C++ workload) |

## Branches

Branch from `develop`, or from `platform/<platform>` for platform-only work:

- `feature/<area>-<topic>`, for example `feature/core-transfer`, `feature/android-qs-tile`
- `fix/<topic>` for bug fixes

Open the pull request against the branch you started from. `main` only receives
releases. Details: [docs/DEVELOPMENT.md#branches](docs/DEVELOPMENT.md#branches).

## Before you open a pull request

Run the same checks CI runs, for the area you changed:

```sh
tools/check.sh core      # or: ui, android, macos, windows, all
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
