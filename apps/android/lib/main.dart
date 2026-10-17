import 'dart:async';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/services.dart';

/// Talks to MainActivity.kt.
const _platform = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/platform',
);

/// Entry point of the Android app. Screens come from ec_ui; this file only
/// connects the Android-specific pieces (see android/app/src/main/kotlin).
void main() {
  final shared = StreamController<List<String>>.broadcast();
  _platform.setMethodCallHandler((call) async {
    if (call.method == 'shared') {
      shared.add(List<String>.from(call.arguments as List));
    }
  });

  runEcApp(
    EcPlatform.android,
    PlatformHooks(
      deviceName: () async =>
          await _platform.invokeMethod<String>('deviceName') ?? 'Android',
      receiveDir: () async =>
          (await _platform.invokeMethod<String>('receiveDir'))!,
      // Permissions, the background receive service and the mDNS multicast lock.
      beforeStart: () => _platform.invokeMethod<void>('prepare'),
      scanInvite: () => _platform.invokeMethod<String>('scanInvite'),
      sharedFiles: () async* {
        // Files shared before the UI was ready, then any later shares.
        final early = await _platform.invokeMethod<List<Object?>>('takeShared');
        if (early != null && early.isNotEmpty) yield List<String>.from(early);
        yield* shared.stream;
      }(),
    ),
  );
}
