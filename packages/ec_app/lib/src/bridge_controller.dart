import 'dart:async';

import 'package:ec_bridge/ec_bridge.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:file_selector/file_selector.dart';
import 'package:url_launcher/url_launcher.dart';

import 'hooks.dart';
import 'notifier.dart';

/// [EcController] backed by the Rust core.
class BridgeController extends EcController {
  BridgeController(this._core, this._hooks, this._notifier) {
    _refresh();
    _events = _core.events().listen(_onEvent);
    _shares = _hooks.sharedFiles?.listen((paths) {
      if (paths.isEmpty) return;
      _pendingShare = [..._pendingShare, ...paths];
      notifyListeners();
    });
  }

  final EcCore _core;
  final PlatformHooks _hooks;
  final Notifier _notifier;
  late final StreamSubscription<CoreEvent> _events;
  StreamSubscription<List<String>>? _shares;
  List<String> _pendingShare = const [];

  List<EcDevice> _devices = const [];
  ReceiveStatus? _receiving;
  final _recent = <String>[];
  var _savedInTransfer = 0;

  @override
  String get myName => _core.myName();

  @override
  String get myShortId => _core.myShortId();

  @override
  List<EcDevice> get devices => _devices;

  @override
  ReceiveStatus? get receiving => _receiving;

  @override
  List<String> get recentFiles => List.unmodifiable(_recent);

  @override
  List<String> get pendingShare => _pendingShare;

  @override
  void dismissShare() {
    _pendingShare = const [];
    notifyListeners();
  }

  @override
  bool get canScan => _hooks.scanInvite != null;

  void _refresh() {
    _devices =
        _core
            .devices()
            .map(
              (d) => EcDevice(
                id: d.id,
                shortId: d.shortId,
                name: d.name,
                online: d.online,
              ),
            )
            .toList()
          ..sort((a, b) {
            if (a.online != b.online) return a.online ? -1 : 1;
            return a.name.compareTo(b.name);
          });
  }

  String _nameOf(String shortId) => _devices
      .firstWhere(
        (d) => d.shortId == shortId,
        orElse: () =>
            EcDevice(id: '', shortId: shortId, name: shortId, online: true),
      )
      .name;

  void _onEvent(CoreEvent event) {
    switch (event) {
      case CoreEvent_DevicesChanged() || CoreEvent_Paired():
        _refresh();
      case CoreEvent_ReceiveStarted(
        :final from,
        :final files,
        :final totalBytes,
        :final alreadyHave,
      ):
        _savedInTransfer = 0;
        _receiving = ReceiveStatus(
          from: _nameOf(from),
          files: files,
          received: alreadyHave.toInt(),
          total: totalBytes.toInt(),
        );
      case CoreEvent_ReceiveProgress(:final received, :final total):
        final current = _receiving;
        if (current != null) {
          _receiving = ReceiveStatus(
            from: current.from,
            files: current.files,
            received: received.toInt(),
            total: total.toInt(),
          );
        }
      case CoreEvent_Saved(:final path):
        _savedInTransfer++;
        _recent.insert(0, path);
        if (_recent.length > 20) _recent.removeLast();
      case CoreEvent_ReceiveFinished():
        final from = _receiving?.from ?? 'a paired device';
        final name = _recent.isEmpty
            ? ''
            : _recent.first.split(RegExp(r'[/\\]')).last;
        unawaited(
          _notifier.received(
            _savedInTransfer == 1
                ? 'Received $name'
                : 'Received $_savedInTransfer files',
            'From $from',
          ),
        );
        _receiving = null;
      case CoreEvent_ReceiveInterrupted():
        _receiving = null;
    }
    notifyListeners();
  }

  @override
  Stream<PairingStep> hostPairing() => _core.hostPairing().map(_pairingStep);

  @override
  Stream<PairingStep> joinPairing(String invite) =>
      _core.joinPairing(invite: invite).map(_pairingStep);

  PairingStep _pairingStep(PairingEvent event) => switch (event) {
    PairingEvent_Invite(:final uri) => PairingInvite(uri),
    PairingEvent_Confirm(:final peerName, :final shortId, :final code) =>
      PairingConfirm(peerName: peerName, shortId: shortId, code: code),
    PairingEvent_Paired(:final name) => PairingDone(name),
    PairingEvent_Failed(:final reason) => PairingFailed(reason),
  };

  @override
  void answerPairing(bool accept) => _core.answerPairing(accept: accept);

  @override
  Future<String?> scanInvite() async => _hooks.scanInvite?.call();

  @override
  Future<List<String>> pickFiles() async {
    final files = await openFiles();
    return files.map((f) => f.path).toList();
  }

  @override
  Stream<SendStatus> send(EcDevice device, List<String> paths) => _core
      .send(deviceId: device.id, paths: paths)
      .map(
        (update) => switch (update) {
          SendUpdate_Progress(:final sent, :final toSend, :final paths) =>
            SendProgress(
              sent: sent.toInt(),
              total: toSend.toInt(),
              paths: paths,
            ),
          SendUpdate_Retrying(:final reason) => SendRetrying(reason),
          SendUpdate_Done(
            :final files,
            :final bytes,
            :final millis,
            :final paths,
          ) =>
            SendDone(
              files: files,
              bytes: bytes.toInt(),
              elapsed: Duration(milliseconds: millis.toInt()),
              paths: paths,
            ),
          SendUpdate_Failed(:final reason) => SendFailed(reason),
        },
      );

  @override
  Future<void> forget(EcDevice device) async {
    _core.forget(deviceId: device.id);
    _refresh();
    notifyListeners();
  }

  @override
  Future<void> open(String path) async {
    final custom = _hooks.openFile;
    if (custom != null) return custom(path);
    await launchUrl(Uri.file(path));
  }

  @override
  void dispose() {
    _events.cancel();
    _shares?.cancel();
    super.dispose();
  }
}
