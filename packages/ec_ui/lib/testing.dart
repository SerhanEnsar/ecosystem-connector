/// A scriptable [EcController] for widget tests and UI previews.
library;

import 'dart:async';

import 'src/controller.dart';
import 'src/model.dart';

/// An in-memory controller. Tests set its fields and push pairing steps.
class FakeController extends EcController {
  FakeController({
    this.myName = 'Test Mac',
    this.myShortId = '1ABE-79D4',
    List<EcDevice>? devices,
    this.canScan = false,
    this.vpnActive = false,
  }) : _devices = devices ?? [];

  @override
  final String myName;

  @override
  final String myShortId;

  @override
  final bool canScan;

  @override
  final bool vpnActive;

  List<EcDevice> _devices;
  ReceiveStatus? _receiving;
  List<String> _pendingShare = [];

  /// Pairing steps emitted by [hostPairing] and [joinPairing].
  final pairing = StreamController<PairingStep>.broadcast();

  /// Answers given to [answerPairing].
  final answers = <bool>[];

  /// Invite returned by [scanInvite].
  String? scanned;

  /// Paths returned by [pickFiles].
  List<String> picked = [];

  /// Sends requested through [send].
  final sends = <(EcDevice, List<String>)>[];

  /// Status emitted by [send].
  final sending = StreamController<SendStatus>.broadcast();

  set devices(List<EcDevice> value) {
    _devices = value;
    notifyListeners();
  }

  set pendingShare(List<String> value) {
    _pendingShare = value;
    notifyListeners();
  }

  @override
  List<String> get pendingShare => _pendingShare;

  @override
  void dismissShare() => pendingShare = [];

  set receiving(ReceiveStatus? value) {
    _receiving = value;
    notifyListeners();
  }

  @override
  List<EcDevice> get devices => _devices;

  @override
  ReceiveStatus? get receiving => _receiving;

  @override
  List<String> get recentFiles => const [];

  @override
  Stream<PairingStep> hostPairing() => pairing.stream;

  @override
  Stream<PairingStep> joinPairing(String invite) => pairing.stream;

  @override
  void answerPairing(bool accept) => answers.add(accept);

  @override
  Future<String?> scanInvite() async => scanned;

  @override
  Future<List<String>> pickFiles() async => picked;

  @override
  Stream<SendStatus> send(EcDevice device, List<String> paths) {
    sends.add((device, paths));
    return sending.stream;
  }

  @override
  Future<void> forget(EcDevice device) async {
    devices = _devices.where((d) => d.id != device.id).toList();
  }

  @override
  Future<void> open(String path) async {}
}
