# ECOnnector for Windows

The Windows app. Screens come from the shared [`ec_ui`](../../packages/ec_ui) package; this
folder holds only what is specific to Windows.

Arrivals (received files, pages and text) show as cards in the window, the same as on macOS and Android, and as a notification while the window is hidden. Planned platform integrations: Tray icon, Explorer "Send to" integration.

## Run

```sh
cd apps/windows
flutter run -d windows
```

Checks: `tools/check.sh windows` from the repository root. Setup: [docs/DEVELOPMENT.md](../../docs/DEVELOPMENT.md#windows).
