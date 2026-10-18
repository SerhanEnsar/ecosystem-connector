# ec_ui

Shared Flutter UI of the ECOnnector apps: the root widget, theme and screens.
Every app in [`apps/`](../../apps) depends on it.

Rules for this package:

- No platform-specific code. Code that differs per platform goes into the app in
  `apps/<platform>/`, or behind an interface that the app implements.
- No `dart:io` platform checks; apps pass their `EcPlatform` in.
