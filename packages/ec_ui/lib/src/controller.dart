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

  /// Flinging from the screen edge, where the platform supports it (Android);
  /// null elsewhere.
  FlingSetup? get flingSetup;

  /// Shows or hides the edge handle.
  Future<void> setFlingHandle(bool on);

  /// Opens the system page where the app may be allowed to draw over other apps.
  Future<void> openOverlaySettings();

  /// Opens the system accessibility settings, to turn on screen access.
  Future<void> openScreenAccessSettings();

  /// A pairing link the app was opened with, waiting to be used.
  String? get pendingInvite;

  /// Marks [pendingInvite] as handled.
  void clearPendingInvite();

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

  /// Sends [content] to [device], which shows it in an arrival card.
  Stream<SendStatus> fling(EcDevice device, FlingContent content);

  /// The folder received files are saved to.
  String get receiveDir;

  /// Whether [receiveDir] is the app's default folder.
  bool get receiveDirIsDefault;

  /// Whether the user can choose another folder on this platform.
  bool get canChooseReceiveDir;

  /// Lets the user pick a folder for received files. Returns false if cancelled;
  /// throws if the folder cannot be used.
  Future<bool> chooseReceiveDir();

  /// Goes back to the default folder for received files.
  Future<void> useDefaultReceiveDir();

  /// Removes a paired device.
  Future<void> forget(EcDevice device);

  /// Opens a received file with the system's default app.
  Future<void> open(String path);
}
