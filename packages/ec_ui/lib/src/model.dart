/// A paired device.
class EcDevice {
  const EcDevice({
    required this.id,
    required this.shortId,
    required this.name,
    required this.online,
    this.linkedFiles = false,
  });

  /// Full id (64 hex characters).
  final String id;

  /// Short id for display, such as `3F9A-C210`.
  final String shortId;

  /// Its name.
  final String name;

  /// Whether it is on the network now.
  final bool online;

  /// Whether files can be linked with it (it runs a version that supports them).
  final bool linkedFiles;
}

/// One step of pairing, as shown to the user.
sealed class PairingStep {
  const PairingStep();
}

/// Show this invite as a QR code and link (when hosting).
class PairingInvite extends PairingStep {
  const PairingInvite(this.uri);
  final String uri;
}

/// Ask the user to compare [code] with the other device.
class PairingConfirm extends PairingStep {
  const PairingConfirm({
    required this.peerName,
    required this.shortId,
    required this.code,
  });
  final String peerName;
  final String shortId;
  final String code;
}

/// Pairing finished.
class PairingDone extends PairingStep {
  const PairingDone(this.name);
  final String name;
}

/// Pairing failed.
class PairingFailed extends PairingStep {
  const PairingFailed(this.reason);
  final String reason;
}

/// Progress of sending files.
sealed class SendStatus {
  const SendStatus();
}

class SendProgress extends SendStatus {
  const SendProgress({
    required this.sent,
    required this.total,
    required this.paths,
  });
  final int sent;
  final int total;
  final int paths;

  double get fraction => total == 0 ? 1 : sent / total;
}

class SendRetrying extends SendStatus {
  const SendRetrying(this.reason);
  final String reason;
}

class SendDone extends SendStatus {
  const SendDone({
    required this.files,
    required this.bytes,
    required this.elapsed,
    required this.paths,
  });
  final int files;
  final int bytes;
  final Duration elapsed;
  final int paths;

  /// Average speed in bytes per second.
  double get bytesPerSecond =>
      elapsed.inMicroseconds == 0 ? 0 : bytes / elapsed.inMicroseconds * 1e6;
}

class SendFailed extends SendStatus {
  const SendFailed(this.reason);
  final String reason;
}

/// What is being received right now.
class ReceiveStatus {
  const ReceiveStatus({
    required this.from,
    required this.files,
    required this.received,
    required this.total,
  });

  /// Name of the sending device.
  final String from;
  final int files;
  final int received;
  final int total;

  double get fraction => total == 0 ? 1 : received / total;
}

/// Formats a byte count, such as `314.6 MB`.
String formatBytes(num bytes) {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  var value = bytes.toDouble();
  var unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit++;
  }
  return unit == 0
      ? '${bytes.toInt()} B'
      : '${value.toStringAsFixed(1)} ${units[unit]}';
}

/// What a fling sends: the thing on screen, shown on the other device in an arrival card.
sealed class FlingContent {
  const FlingContent();

  /// What the header shows while dragging, such as `Web page · QUIC`.
  String get label;
}

/// A file, opened at [page] if it has pages.
class FlingFile extends FlingContent {
  const FlingFile(this.path, {this.page});
  final String path;
  final int? page;

  String get name => path.split(RegExp(r'[/\\]')).last;

  @override
  String get label =>
      page == null ? 'File · $name' : 'File · $name · page $page';
}

/// Files shared from another app, sent together.
class FlingFiles extends FlingContent {
  const FlingFiles(this.paths);
  final List<String> paths;

  /// The single file's name, or the number of files.
  String get name => paths.length == 1
      ? paths.single.split(RegExp(r'[/\\]')).last
      : '${paths.length} files';

  @override
  String get label => paths.length == 1 ? 'File · $name' : name;
}

/// Text, offered to copy on the other device.
class FlingText extends FlingContent {
  const FlingText(this.text);
  final String text;

  @override
  String get label => 'Text';
}

/// A web page, opened in the browser. [preview] is a screenshot of it, shown while
/// flinging.
class FlingPage extends FlingContent {
  const FlingPage(this.url, {this.title = '', this.preview});
  final String url;
  final String title;
  final String? preview;

  @override
  String get label => 'Web page · ${title.isEmpty ? url : title}';
}

/// A screenshot of the screen, saved to [path].
class FlingScreenshot extends FlingContent {
  const FlingScreenshot(this.path);
  final String path;

  @override
  String get label => 'Screenshot';
}

/// What flinging from the screen edge needs on this device (Android).
class FlingSetup {
  const FlingSetup({
    required this.handleOn,
    required this.canDrawOverlays,
    required this.screenAccess,
  });

  /// The user turned the edge handle on.
  final bool handleOn;

  /// The app may draw over other apps (needed for the handle).
  final bool canDrawOverlays;

  /// The accessibility service is on (needed for screenshots and web pages).
  final bool screenAccess;

  /// Everything is in place.
  bool get ready => handleOn && canDrawOverlays && screenAccess;
}

/// Where a linked file stands on this device.
enum EcLinkedState {
  /// Up to date.
  current,

  /// A newer version is arriving.
  updating,

  /// Changed on two devices; the user chooses.
  conflict,

  /// Cannot be found; the link is kept.
  missing,
}

/// The two versions of a linked file that changed on two devices.
class EcLinkedConflict {
  const EcLinkedConflict({
    required this.mineDevice,
    required this.mineAt,
    required this.mineSize,
    required this.theirsDevice,
    required this.theirsAt,
    required this.theirsSize,
    required this.theirsPath,
  });

  /// This device's name.
  final String mineDevice;
  final DateTime mineAt;
  final int mineSize;

  /// The other device's name.
  final String theirsDevice;
  final DateTime theirsAt;
  final int theirsSize;

  /// Where the other version waits.
  final String theirsPath;
}

/// A file kept current on several devices.
class EcLinkedFile {
  const EcLinkedFile({
    required this.id,
    required this.name,
    required this.path,
    required this.devices,
    required this.state,
    required this.size,
    this.conflict,
  });

  final String id;

  /// Its name on this device.
  final String name;

  /// Where it is on this device.
  final String path;

  /// Names of the devices it is linked with.
  final List<String> devices;
  final EcLinkedState state;
  final int size;

  /// Set when [state] is [EcLinkedState.conflict].
  final EcLinkedConflict? conflict;
}

/// One entry of a linked file's log.
class EcLinkedLog {
  const EcLinkedLog({
    required this.at,
    required this.device,
    required this.text,
  });

  final DateTime at;

  /// The device it happened on.
  final String device;

  /// What happened, such as "changed".
  final String text;
}

/// A time for display: "today 14:02", "yesterday 09:30" or "2026-10-12 18:45".
String formatWhen(DateTime at, {DateTime? now}) {
  final today = now ?? DateTime.now();
  String two(int n) => n.toString().padLeft(2, '0');
  final clock = '${two(at.hour)}:${two(at.minute)}';
  final day = DateTime(at.year, at.month, at.day);
  final days = DateTime(
    today.year,
    today.month,
    today.day,
  ).difference(day).inDays;
  if (days == 0) return 'today $clock';
  if (days == 1) return 'yesterday $clock';
  return '${at.year}-${two(at.month)}-${two(at.day)} $clock';
}
