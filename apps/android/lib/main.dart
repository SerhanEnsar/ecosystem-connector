import 'dart:async';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'fling_layer.dart';

/// Talks to MainActivity.kt.
const _platform = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/platform',
);

/// Entry point of the Android app. Screens come from ec_ui; this file only
/// connects the Android-specific pieces (see android/app/src/main/kotlin).
void main() {
  // The platform channel below needs the binding before runEcApp starts it.
  WidgetsFlutterBinding.ensureInitialized();
  final shared = StreamController<List<String>>.broadcast();
  final invites = StreamController<String>.broadcast();
  _platform.setMethodCallHandler((call) async {
    switch (call.method) {
      case 'shared':
        shared.add(List<String>.from(call.arguments as List));
      case 'invite':
        invites.add(call.arguments as String);
    }
  });

  runEcApp(
    EcPlatform.android,
    _hooks(
      // Permissions, the background receive service and the mDNS multicast lock.
      beforeStart: () => _platform.invokeMethod<void>('prepare'),
      incomingInvites: () async* {
        // The link the app was opened with, then any later ones.
        final first = await _platform.invokeMethod<String>('takeInvite');
        if (first != null) yield first;
        yield* invites.stream;
      }(),
      sharedFiles: () async* {
        // Files shared before the UI was ready, then any later shares.
        final early = await _platform.invokeMethod<List<Object?>>('takeShared');
        if (early != null && early.isNotEmpty) yield List<String>.from(early);
        yield* shared.stream;
      }(),
    ),
  );
}

/// Entry point of the fling layer: a second Flutter engine that FlingHandle.kt starts
/// early and shows over other apps while the edge handle is held.
@pragma('vm:entry-point')
Future<void> flingMain() async {
  WidgetsFlutterBinding.ensureInitialized();
  // Joins the core the app already started; the app posts the notifications and
  // watches the VPN and permissions, so this engine needs only the basics, which
  // FlingHandle.kt answers on the fling channel.
  const fling = MethodChannel('io.github.serhanensar.ecosystemconnector/fling');
  final controller = await startController(
    PlatformHooks(
      deviceName: () async =>
          await fling.invokeMethod<String>('deviceName') ?? 'Android',
      receiveDir: () async => (await fling.invokeMethod<String>('receiveDir'))!,
    ),
    notify: false,
  );
  runApp(FlingLayer(controller: controller));
}

PlatformHooks _hooks({
  Future<void> Function()? beforeStart,
  Stream<List<String>>? sharedFiles,
  Stream<String>? incomingInvites,
}) => PlatformHooks(
  deviceName: () async =>
      await _platform.invokeMethod<String>('deviceName') ?? 'Android',
  receiveDir: () async => (await _platform.invokeMethod<String>('receiveDir'))!,
  beforeStart: beforeStart,
  scanInvite: () => _platform.invokeMethod<String>('scanInvite'),
  sharedFiles: sharedFiles,
  incomingInvites: incomingInvites,
  vpnActive: () async =>
      await _platform.invokeMethod<bool>('vpnActive') ?? false,
  fling: FlingHooks(
    status: () async {
      final s = await _platform.invokeMapMethod<String, bool>('flingStatus');
      return FlingSetup(
        handleOn: s?['handleOn'] ?? false,
        canDrawOverlays: s?['canDrawOverlays'] ?? false,
        screenAccess: s?['screenAccess'] ?? false,
      );
    },
    setHandle: (on) => _platform.invokeMethod<void>('setFlingHandle', on),
    openOverlaySettings: () =>
        _platform.invokeMethod<void>('openOverlaySettings'),
    openAccessibilitySettings: () =>
        _platform.invokeMethod<void>('openAccessibilitySettings'),
  ),
);
