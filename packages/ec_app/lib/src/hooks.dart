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
}
