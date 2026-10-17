/// A paired device.
class EcDevice {
  const EcDevice({
    required this.id,
    required this.shortId,
    required this.name,
    required this.online,
  });

  /// Full id (64 hex characters).
  final String id;

  /// Short id for display, such as `3F9A-C210`.
  final String shortId;

  /// Its name.
  final String name;

  /// Whether it is on the network now.
  final bool online;
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

/// What a fling sends: the thing on screen, opened on the other device on arrival.
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

/// A web page, opened in the browser.
class FlingPage extends FlingContent {
  const FlingPage(this.url, {this.title = ''});
  final String url;
  final String title;

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
