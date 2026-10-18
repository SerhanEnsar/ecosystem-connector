# ECOnnector for Android

The Android app. Screens come from the shared [`ec_ui`](../../packages/ec_ui) package; this
folder holds only what is specific to Android.

Planned platform integrations: Quick Settings tile, share sheet target, foreground service, notification actions.

## Run

```sh
cd apps/android
flutter run -d <device-id>   # list devices with: flutter devices
```

Checks: `tools/check.sh android` from the repository root. Setup: [docs/DEVELOPMENT.md](../../docs/DEVELOPMENT.md#android).
