import 'dart:async';
import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/widgets.dart';
import 'package:path_provider/path_provider.dart';

import 'arrivals.dart';

/// Arrival cards for the window, drawn by ECOnnectorApp.
final _arrivals = StreamController<ArrivalEntry>.broadcast();

/// Entry point of the Windows app. Screens come from ec_ui; this file only connects
/// the Windows-specific pieces.
void main() {
  runEcApp(
    EcPlatform.windows,
    PlatformHooks(
      chooseFolder: pickFolder,
      deviceName: () async => Platform.localHostname.split('.').first,
      receiveDir: () async {
        final downloads = await getDownloadsDirectory();
        final dir = Directory(
          '${downloads!.path}${Platform.pathSeparator}ECOnnector',
        );
        await dir.create(recursive: true);
        return dir.path;
      },
      // The card shows in the window while it is on screen; when it is minimised or
      // hidden, the notification (see Notifier) is shown instead.
      showArrival: (arrival) async {
        if (!_windowVisible()) return false;
        _arrivals.add(windowsArrival(arrival));
        return true;
      },
    ),
    arrivals: _arrivals.stream,
  );
}

bool _windowVisible() {
  final state = WidgetsBinding.instance.lifecycleState;
  return state == null ||
      state == AppLifecycleState.resumed ||
      state == AppLifecycleState.inactive;
}
