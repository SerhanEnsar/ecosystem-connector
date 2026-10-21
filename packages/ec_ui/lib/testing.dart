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

  FlingContent? _pendingItem;

  set pendingItem(FlingContent? value) {
    _pendingItem = value;
    notifyListeners();
  }

  @override
  FlingContent? get pendingItem => _pendingItem;

  @override
  void dismissShare() {
    _pendingItem = null;
    pendingShare = [];
  }

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

  /// Flings requested through [fling].
  final flings = <(EcDevice, FlingContent)>[];

  @override
  Stream<SendStatus> fling(EcDevice device, FlingContent content) {
    flings.add((device, content));
    return sending.stream;
  }

  FlingSetup? _flingSetup;

  set flingSetup(FlingSetup? value) {
    _flingSetup = value;
    notifyListeners();
  }

  @override
  FlingSetup? get flingSetup => _flingSetup;

  String? _pendingInvite;

  set pendingInvite(String? value) {
    _pendingInvite = value;
    notifyListeners();
  }

  @override
  String? get pendingInvite => _pendingInvite;

  @override
  void clearPendingInvite() => _pendingInvite = null;

  /// Calls made to the fling setup methods, in order.
  final setupCalls = <String>[];

  @override
  Future<void> setFlingHandle(bool on) async => setupCalls.add('handle:$on');

  @override
  Future<void> openOverlaySettings() async => setupCalls.add('overlay');

  @override
  Future<void> openScreenAccessSettings() async => setupCalls.add('access');

  /// The default folder, and what [chooseReceiveDir] picks next (null cancels).
  String defaultReceiveDir = '/Users/test/Downloads/ECOnnector';
  String? folderToPick;

  @override
  bool canChooseReceiveDir = true;

  String? _chosenDir;

  @override
  String get receiveDir => _chosenDir ?? defaultReceiveDir;

  @override
  bool get receiveDirIsDefault => _chosenDir == null;

  @override
  Future<bool> chooseReceiveDir() async {
    if (folderToPick == null) return false;
    _chosenDir = folderToPick;
    notifyListeners();
    return true;
  }

  @override
  Future<void> useDefaultReceiveDir() async {
    _chosenDir = null;
    notifyListeners();
  }

  @override
  Future<void> forget(EcDevice device) async {
    devices = _devices.where((d) => d.id != device.id).toList();
  }

  @override
  Future<void> open(String path) async => opened.add(path);

  @override
  bool sendLinked = false;

  /// Linked sends requested through [sendLinkedFile].
  final linkedSends = <(EcDevice, String)>[];

  @override
  Stream<SendStatus> sendLinkedFile(EcDevice device, String path) {
    linkedSends.add((device, path));
    return sending.stream;
  }

  List<EcLinkedFile> _linked = [];

  set linkedFiles(List<EcLinkedFile> value) {
    _linked = value;
    notifyListeners();
  }

  @override
  List<EcLinkedFile> get linkedFiles => _linked;

  /// Returned by [linkedLog].
  List<EcLinkedLog> log = [];

  @override
  Future<List<EcLinkedLog>> linkedLog(String id) async => log;

  final _dismissed = <String>{};

  @override
  EcLinkedFile? get pendingConflict {
    for (final file in _linked) {
      if (file.state == EcLinkedState.conflict &&
          !_dismissed.contains(file.id)) {
        return file;
      }
    }
    return null;
  }

  @override
  void dismissConflict(String id) {
    _dismissed.add(id);
    notifyListeners();
  }

  /// Calls made to the linked files methods, such as `resolve:id:theirs`.
  final linkedCalls = <String>[];

  @override
  Future<void> resolveConflict(String id, {required bool keepTheirs}) async =>
      linkedCalls.add('resolve:$id:${keepTheirs ? 'theirs' : 'mine'}');

  @override
  Future<void> deleteConflict(String id) async =>
      linkedCalls.add('delete-conflict:$id');

  @override
  Future<void> leaveLink(String id) async => linkedCalls.add('leave:$id');

  @override
  Future<void> locateLinked(String id) async => linkedCalls.add('locate:$id');

  @override
  Future<void> linkFile(EcDevice device) async =>
      linkedCalls.add('link:${device.id}');

  @override
  Future<void> refreshLinked() async => linkedCalls.add('refresh');

  @override
  bool linkedNeedsAccess = false;

  @override
  Future<void> openLinkedAccessSettings() async => setupCalls.add('files');

  /// Paths given to [open].
  final opened = <String>[];
}
