import 'package:ec_ui/ec_ui.dart';

/// Flinging from the screen edge (Android).
class FlingHooks {
  const FlingHooks({
    required this.status,
    required this.setHandle,
    required this.openOverlaySettings,
    required this.openAccessibilitySettings,
  });

  final Future<FlingSetup> Function() status;
  final Future<void> Function(bool on) setHandle;
  final Future<void> Function() openOverlaySettings;
  final Future<void> Function() openAccessibilitySettings;
}

/// What each platform app supplies to the shared app code.
class PlatformHooks {
  const PlatformHooks({
    required this.deviceName,
    required this.receiveDir,
    this.beforeStart,
    this.scanInvite,
    this.openFile,
    this.sharedFiles,
    this.vpnActive,
    this.opensFlings = false,
    this.fling,
    this.incomingInvites,
  });

  /// The name this device gets when it is set up for the first time.
  final Future<String> Function() deviceName;

  /// Folder where received files are saved.
  final Future<String> Function() receiveDir;

  /// Runs before the core starts, for example to request permissions or keep the
  /// process alive in the background.
  final Future<void> Function()? beforeStart;

  /// Scans a pairing QR code with the camera. Null where there is no camera flow.
  final Future<String?> Function()? scanInvite;

  /// Opens a received file. Null uses the default (desktop: the system's handler).
  final Future<void> Function(String path)? openFile;

  /// Files shared to the app from other apps (Android share sheet).
  final Stream<List<String>>? sharedFiles;

  /// Whether a VPN is on, which can stop other devices from reaching this one.
  final Future<bool> Function()? vpnActive;

  /// Whether flung files and pages are opened as soon as they arrive. True on
  /// desktops; phones and tablets only notify, because Android does not let apps
  /// open things from the background.
  final bool opensFlings;

  /// Flinging from the screen edge, where the platform supports it.
  final FlingHooks? fling;

  /// Pairing links the app was opened with (scanned with the system camera).
  final Stream<String>? incomingInvites;
}
