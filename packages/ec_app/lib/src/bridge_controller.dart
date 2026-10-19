import 'dart:async';
import 'dart:io';

import 'package:ec_bridge/ec_bridge.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/widgets.dart';
import 'package:url_launcher/url_launcher.dart';

import 'hooks.dart';
import 'notifier.dart';

/// [EcController] backed by the Rust core.
class BridgeController extends EcController with WidgetsBindingObserver {
  BridgeController(this._core, this._hooks, this._notifier) {
    _refresh();
    _refreshLinked();
    unawaited(_restoreAccess());
    // Android may not report changes in shared storage; check linked files often
    // while the app is in front. The receive service checks less often.
    if (Platform.isAndroid) _core.setLinkedCheckInterval(seconds: 5);
    _events = _core.events().listen(_onEvent);
    _invites = _hooks.incomingInvites?.listen((invite) {
      _pendingInvite = invite;
      notifyListeners();
    });
    _shares = _hooks.sharedFiles?.listen((paths) {
      if (paths.isEmpty) return;
      // The same file shared twice is sent once.
      _pendingShare = {..._pendingShare, ...paths}.toList();
      notifyListeners();
    });
    _items = _hooks.sharedItems?.listen((item) {
      _pendingItem = item;
      notifyListeners();
    });
    if (_hooks.vpnActive != null ||
        _hooks.fling != null ||
        Platform.isAndroid) {
      unawaited(_checkPlatform());
      // A VPN or a permission can change at any time.
      _vpnTimer = Timer.periodic(
        const Duration(seconds: 15),
        (_) => _checkPlatform(),
      );
      WidgetsBinding.instance.addObserver(this);
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // Back from the system settings: show the new state right away.
    if (state == AppLifecycleState.resumed) unawaited(_checkPlatform());
    if (Platform.isAndroid) {
      _core.setLinkedCheckInterval(
        seconds: state == AppLifecycleState.resumed ? 5 : 30,
      );
    }
  }

  Future<void> _checkPlatform() async {
    await _checkVpn();
    final fling = _hooks.fling;
    if (fling != null) {
      final setup = await fling.status();
      final old = _flingSetup;
      if (old == null ||
          old.handleOn != setup.handleOn ||
          old.canDrawOverlays != setup.canDrawOverlays ||
          old.screenAccess != setup.screenAccess) {
        _flingSetup = setup;
        notifyListeners();
      }
    }
  }

  Future<void> _checkVpn() async {
    final check = _hooks.vpnActive;
    if (check == null) return;
    final active = await check();
    if (active != _vpnActive) {
      _vpnActive = active;
      notifyListeners();
    }
  }

  final EcCore _core;
  final PlatformHooks _hooks;
  final Notifier _notifier;
  late final StreamSubscription<CoreEvent> _events;
  StreamSubscription<List<String>>? _shares;
  StreamSubscription<FlingContent>? _items;
  FlingContent? _pendingItem;
  StreamSubscription<String>? _invites;
  String? _pendingInvite;
  List<String> _pendingShare = const [];
  Timer? _vpnTimer;
  FlingSetup? _flingSetup;
  var _vpnActive = false;

  List<EcDevice> _devices = const [];
  List<EcLinkedFile> _linked = const [];
  final _dismissedConflicts = <String>{};
  ReceiveStatus? _receiving;
  final _recent = <String>[];
  final _savedInTransfer = <String>[];

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
  FlingContent? get pendingItem => _pendingItem;

  @override
  void dismissShare() {
    _pendingShare = const [];
    _pendingItem = null;
    notifyListeners();
  }

  @override
  bool get vpnActive => _vpnActive;

  @override
  String? get pendingInvite => _pendingInvite;

  @override
  void clearPendingInvite() => _pendingInvite = null;

  @override
  FlingSetup? get flingSetup => _flingSetup;

  @override
  Future<void> setFlingHandle(bool on) async {
    await _hooks.fling?.setHandle(on);
    await _checkPlatform();
  }

  @override
  Future<void> openOverlaySettings() async =>
      _hooks.fling?.openOverlaySettings();

  @override
  Future<void> openScreenAccessSettings() async =>
      _hooks.fling?.openAccessibilitySettings();

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
                linkedFiles: d.linkedFiles,
              ),
            )
            .toList()
          ..sort((a, b) {
            if (a.online != b.online) return a.online ? -1 : 1;
            return a.name.compareTo(b.name);
          });
  }

  void _refreshLinked() {
    _linked = _core.linkedFiles().map(_linkedFile).toList()
      ..sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
  }

  EcLinkedFile _linkedFile(LinkedView f) => EcLinkedFile(
    id: f.id,
    name: f.name,
    path: f.path,
    devices: f.devices,
    state: switch (f.state) {
      LinkedState.current => EcLinkedState.current,
      LinkedState.updating => EcLinkedState.updating,
      LinkedState.conflict => EcLinkedState.conflict,
      LinkedState.missing => EcLinkedState.missing,
    },
    size: f.size.toInt(),
    conflict: switch (f.conflict) {
      final c? => EcLinkedConflict(
        mineDevice: c.mineDevice,
        mineAt: DateTime.fromMillisecondsSinceEpoch(c.mineAtMs.toInt()),
        mineSize: c.mineSize.toInt(),
        theirsDevice: c.theirsDevice,
        theirsAt: DateTime.fromMillisecondsSinceEpoch(c.theirsAtMs.toInt()),
        theirsSize: c.theirsSize.toInt(),
        theirsPath: c.theirsPath,
      ),
      null => null,
    },
  );

  String _linkedName(String id) =>
      _linked.where((f) => f.id == id).firstOrNull?.name ?? 'A linked file';

  /// Opens linked files kept from an earlier run again (macOS bookmarks) and tells
  /// the core where any of them went meanwhile.
  Future<void> _restoreAccess() async {
    final restore = _hooks.restoreAccess;
    if (restore == null) return;
    final places = await restore();
    for (final file in _linked) {
      final now = places[file.id];
      if (now != null && now != file.path) {
        _core.relocateLinked(id: file.id, path: now);
      }
    }
    await _emptyTrash();
  }

  /// Moves the versions the user chose not to keep to the system Trash.
  Future<void> _emptyTrash() async {
    final trash = _hooks.trash;
    if (trash == null) return;
    for (final path in _core.takePendingTrash()) {
      await trash(path);
    }
  }

  Future<void> _onLinked(String id, LinkedEventKind kind) async {
    final name = _linkedName(id);
    switch (kind) {
      case LinkedEventKind_Arrived(:final path, :final from):
        await _hooks.keepAccess?.call(id, path);
        await _arrived(
          ArrivalInfo.files(
            [path],
            from: _nameOf(from),
            size: _linked.where((f) => f.id == id).firstOrNull?.size ?? 0,
            linked: true,
          ),
        );
      case LinkedEventKind_Conflict():
        _dismissedConflicts.remove(id);
        await _notifier.linked(
          id,
          '$name changed on two devices',
          'Choose which version to keep.',
        );
      case LinkedEventKind_PeerLeft(:final device):
        await _notifier.linked(
          id,
          '${_nameOf(device)} left the link of $name',
          'The file stays here and on your other devices.',
        );
      case LinkedEventKind_Left():
        await _notifier.linked(
          id,
          '$name is no longer linked',
          'It was deleted here; your other devices keep it.',
        );
      case LinkedEventKind_Missing():
        await _notifier.linked(
          id,
          '$name cannot be found',
          'Show where it is in Settings → Linked files, or leave the link.',
        );
      case LinkedEventKind_Moved():
        final file = _linked.where((f) => f.id == id).firstOrNull;
        if (file != null) await _hooks.keepAccess?.call(id, file.path);
      case LinkedEventKind_Resolved():
        await _emptyTrash();
      case LinkedEventKind_Updated() ||
          LinkedEventKind_Found() ||
          LinkedEventKind_Error():
        break;
    }
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
        _savedInTransfer.clear();
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
        _savedInTransfer.add(path);
        _recent.insert(0, path);
        if (_recent.length > 20) _recent.removeLast();
      case CoreEvent_FlingUrl(:final from, :final url, :final title):
        unawaited(
          _arrived(ArrivalInfo.page(url, title: title, from: _nameOf(from))),
        );
      case CoreEvent_FlingText(:final from, :final text):
        unawaited(_arrived(ArrivalInfo.text(text, from: _nameOf(from))));
      case CoreEvent_ReceiveFinished():
        final paths = List<String>.of(_savedInTransfer);
        if (paths.isNotEmpty) {
          unawaited(
            _arrived(
              ArrivalInfo.files(
                paths,
                from: _receiving?.from ?? 'a paired device',
                size: _receiving?.total ?? 0,
              ),
            ),
          );
        }
        _receiving = null;
      case CoreEvent_ReceiveInterrupted():
        _receiving = null;
      case CoreEvent_Linked(:final id, :final kind):
        _refreshLinked();
        unawaited(_onLinked(id, kind));
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
  String get receiveDir => _core.receiveDir();

  @override
  bool get receiveDirIsDefault => _core.receiveDirIsDefault();

  @override
  bool get canChooseReceiveDir => _hooks.chooseFolder != null;

  @override
  Future<bool> chooseReceiveDir() async {
    final path = await _hooks.chooseFolder?.call();
    if (path == null) return false;
    _core.setReceiveDir(path: path);
    notifyListeners();
    return true;
  }

  @override
  Future<void> useDefaultReceiveDir() async {
    _core.setReceiveDir();
    await _hooks.folderReset?.call();
    notifyListeners();
  }

  /// Shows the arrival card where the platform has one, a notification otherwise.
  Future<void> _arrived(ArrivalInfo arrival) async {
    final show = _hooks.showArrival;
    if (show != null && await show(arrival)) return;
    await _notifier.received(arrival.displayTitle, 'From ${arrival.from}');
  }

  @override
  Stream<SendStatus> send(EcDevice device, List<String> paths) =>
      _sendStatus(_core.send(deviceId: device.id, paths: paths, open: false));

  @override
  Stream<SendStatus> fling(EcDevice device, FlingContent content) =>
      switch (content) {
        FlingFile(:final path, :final page) => _sendStatus(
          _core.send(
            deviceId: device.id,
            paths: [path],
            open: true,
            page: page,
          ),
        ),
        FlingFiles(:final paths) => _sendStatus(
          _core.send(deviceId: device.id, paths: paths, open: true),
        ),
        FlingScreenshot(:final path) => _sendStatus(
          _core.send(deviceId: device.id, paths: [path], open: true),
        ),
        FlingPage(:final url, :final title) => _flingItem(
          () => _core.flingUrl(deviceId: device.id, url: url, title: title),
        ),
        FlingText(:final text) => _flingItem(
          () => _core.flingText(deviceId: device.id, text: text),
        ),
      };

  /// A page or text: one short message, done when the other device accepts it.
  Stream<SendStatus> _flingItem(Future<void> Function() send) async* {
    final watch = Stopwatch()..start();
    try {
      await send();
      yield SendDone(files: 0, bytes: 0, elapsed: watch.elapsed, paths: 1);
    } catch (e) {
      yield SendFailed(_reason(e));
    }
  }

  Stream<SendStatus> _sendStatus(Stream<SendUpdate> updates) => updates.map(
    (update) => switch (update) {
      SendUpdate_Progress(:final sent, :final toSend, :final paths) =>
        SendProgress(sent: sent.toInt(), total: toSend.toInt(), paths: paths),
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
  bool sendLinked = false;

  @override
  Stream<SendStatus> sendLinkedFile(EcDevice device, String path) async* {
    yield const SendProgress(sent: 0, total: 0, paths: 0);
    final watch = Stopwatch()..start();
    try {
      final id = await _core.sendLinked(
        deviceId: device.id,
        path: path,
        copyIntoLinked: _hooks.copyIntoLinked,
      );
      _refreshLinked();
      final file = _linked.where((f) => f.id == id).firstOrNull;
      if (file != null) await _hooks.keepAccess?.call(id, file.path);
      notifyListeners();
      yield SendDone(
        files: 1,
        bytes: file?.size ?? 0,
        elapsed: watch.elapsed,
        paths: 1,
      );
    } catch (e) {
      yield SendFailed(_reason(e));
    }
  }

  @override
  List<EcLinkedFile> get linkedFiles => _linked;

  @override
  Future<List<EcLinkedLog>> linkedLog(String id) async => _core
      .linkedLog(id: id)
      .map(
        (e) => EcLinkedLog(
          at: DateTime.fromMillisecondsSinceEpoch(e.atMs.toInt()),
          device: e.device,
          text: e.text,
        ),
      )
      .toList();

  @override
  EcLinkedFile? get pendingConflict => _linked
      .where(
        (f) =>
            f.state == EcLinkedState.conflict &&
            !_dismissedConflicts.contains(f.id),
      )
      .firstOrNull;

  @override
  void dismissConflict(String id) {
    _dismissedConflicts.add(id);
    notifyListeners();
  }

  @override
  Future<void> resolveConflict(String id, {required bool keepTheirs}) async {
    _core.resolveConflict(id: id, keepTheirs: keepTheirs);
    await _afterLinkedChange();
  }

  @override
  Future<void> deleteConflict(String id) async {
    _core.deleteConflict(id: id);
    await _afterLinkedChange();
  }

  @override
  Future<void> leaveLink(String id) async {
    _core.leaveLink(id: id);
    await _afterLinkedChange();
  }

  @override
  Future<void> locateLinked(String id) async {
    final file = await openFile();
    if (file == null) return;
    _core.relocateLinked(id: id, path: file.path);
    await _hooks.keepAccess?.call(id, file.path);
    await _afterLinkedChange();
  }

  @override
  Future<void> linkFile(EcDevice device) async {
    final file = await openFile();
    if (file == null) return;
    await sendLinkedFile(device, file.path).last;
  }

  Future<void> _afterLinkedChange() async {
    await _emptyTrash();
    _refreshLinked();
    notifyListeners();
  }

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
    _items?.cancel();
    _invites?.cancel();
    _vpnTimer?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }
}

/// The first line of an error from the core, without the exception wrapper and the
/// backtrace, fit to show to the user.
String _reason(Object error) {
  var text = '$error';
  const wrapper = 'AnyhowException(';
  if (text.startsWith(wrapper)) text = text.substring(wrapper.length);
  return text.split('\n').first.trim();
}
