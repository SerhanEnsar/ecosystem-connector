# ECOnnector for macOS

The macOS app. Screens come from the shared [`ec_ui`](../../packages/ec_ui) package; this
folder holds only what is specific to macOS.

Planned platform integrations: Menu bar icon, front-window document for handoff (Accessibility API), Finder integration.

## Run

```sh
cd apps/macos
flutter run -d macos
```

Checks: `tools/check.sh macos` from the repository root. Setup: [docs/DEVELOPMENT.md](../../docs/DEVELOPMENT.md#macos).
