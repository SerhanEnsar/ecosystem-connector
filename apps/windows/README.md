# Ecosystem Connector for Windows

The Windows app. Screens come from the shared [`ec_ui`](../../packages/ec_ui) package; this
folder holds only what is specific to Windows.

Planned platform integrations: Tray icon, toast notifications, Explorer "Send to" integration.

## Run

```sh
cd apps/windows
flutter run -d windows
```

Checks: `tools/check.sh windows` from the repository root. Setup: [docs/DEVELOPMENT.md](../../docs/DEVELOPMENT.md#windows).
