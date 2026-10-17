import 'package:flutter/foundation.dart';

import 'model.dart';

/// Everything the screens need from the running device.
///
/// `ec_app` implements this on top of the Rust core; tests use a fake. Screens only
/// read state and call these methods, so they contain no platform code.
abstract class EcController extends ChangeNotifier {
  /// This device's name.
  String get myName;

  /// This device's short id.
  String get myShortId;

  /// Paired devices, online first.
  List<EcDevice> get devices;

  /// The transfer being received, if any.
  ReceiveStatus? get receiving;

  /// Recently received files (paths), newest first.
  List<String> get recentFiles;

  /// Files shared from another app, waiting for the user to pick a device.
  List<String> get pendingShare;

  /// Drops [pendingShare] without sending.
  void dismissShare();

  /// Whether a VPN is on, which can stop other devices from reaching this one.
  bool get vpnActive;

  /// Whether this platform can scan a QR code with a camera.
  bool get canScan;

  /// Shows an invite and waits for another device to use it.
  Stream<PairingStep> hostPairing();

  /// Pairs with the device that shows [invite].
  Stream<PairingStep> joinPairing(String invite);

  /// Answers the last [PairingConfirm].
  void answerPairing(bool accept);

  /// Opens the camera and returns a scanned invite, or null if cancelled.
  Future<String?> scanInvite();

  /// Lets the user pick files; returns their paths, or an empty list if cancelled.
  Future<List<String>> pickFiles();

  /// Sends [paths] to [device].
  Stream<SendStatus> send(EcDevice device, List<String> paths);

  /// Sends [content] to [device], which opens it on arrival.
  Stream<SendStatus> fling(EcDevice device, FlingContent content);

  /// Removes a paired device.
  Future<void> forget(EcDevice device);

  /// Opens a received file with the system's default app.
  Future<void> open(String path);
}
