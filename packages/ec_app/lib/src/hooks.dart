import 'package:ec_ui/ec_ui.dart';
import 'package:file_selector/file_selector.dart';

/// The system folder picker, for [PlatformHooks.chooseFolder] where nothing more is
/// needed to keep access to the folder (Windows).
Future<String?> pickFolder() =>
    getDirectoryPath(confirmButtonText: 'Save received files here');

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

/// Something that arrived from another device, for the arrival card.
class ArrivalInfo {
  ArrivalInfo.files(this.paths, {required this.from, this.size = 0})
    : url = null,
      title = paths.length == 1 ? '' : '${paths.length} files';

  const ArrivalInfo.page(
    String this.url, {
    required this.title,
    required this.from,
  }) : paths = const [],
       size = 0;

  /// Saved files (empty for a page).
  final List<String> paths;

  /// The page address (null for files).
  final String? url;

  /// What the card shows as its title.
  final String title;

  /// Name of the sending device.
  final String from;

  /// Total size of the files in bytes.
  final int size;

  /// The card's title: the file name, the number of files, or the page title.
  String get displayTitle {
    if (url != null) return title.isEmpty ? url! : title;
    if (paths.length == 1) return paths.single.split(RegExp(r'[/\\]')).last;
    return title;
  }

  Map<String, Object?> toMap() => {
    'paths': paths,
    'url': url,
    'title': displayTitle,
    'from': from,
    'size': size,
  };
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
    this.showArrival,
    this.fling,
    this.incomingInvites,
    this.chooseFolder,
    this.folderReset,
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

  /// Shows the arrival card for received files or a flung page (the drop that
  /// spreads into a card at the top of the screen) and returns whether it did; if
  /// not, a notification is shown. Nothing is ever opened automatically; the card
  /// offers to.
  final Future<bool> Function(ArrivalInfo arrival)? showArrival;

  /// Flinging from the screen edge, where the platform supports it.
  final FlingHooks? fling;

  /// Pairing links the app was opened with (scanned with the system camera).
  final Stream<String>? incomingInvites;

  /// Lets the user pick a folder for received files and returns its path, or null
  /// if cancelled. The app must be able to write there afterwards, also after a
  /// restart (macOS keeps a security-scoped bookmark). Null where the folder is fixed.
  final Future<String?> Function()? chooseFolder;

  /// Called when the user goes back to the default folder, so a platform can let go
  /// of the folder chosen before.
  final Future<void> Function()? folderReset;
}
