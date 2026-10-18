import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';

const _arrival = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/arrival',
);

/// FolderAccess.swift keeps access to a receive folder outside Downloads.
const _folder = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/folder',
);

/// Entry point of the macOS app. Screens come from ec_ui; this file only connects
/// the macOS-specific pieces.
void main() {
  runEcApp(
    EcPlatform.macos,
    PlatformHooks(
      // Before the core starts, so it can still save to the folder chosen last time.
      beforeStart: () => _folder.invokeMethod<void>('restore'),
      chooseFolder: () => _folder.invokeMethod<String>('choose'),
      folderReset: () => _folder.invokeMethod<void>('forget'),
      // ArrivalPanel.swift draws the card.
      showArrival: (arrival) async {
        await _arrival.invokeMethod<void>('show', arrival.toMap());
        return true;
      },
      deviceName: () async => Platform.localHostname.split('.').first,
      receiveDir: () async {
        final downloads = await getDownloadsDirectory();
        final dir = Directory(
          '${downloads!.path}${Platform.pathSeparator}ECOnnector',
        );
        await dir.create(recursive: true);
        return dir.path;
      },
    ),
  );
}
