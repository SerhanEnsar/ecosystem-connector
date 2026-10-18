import 'dart:async';
import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:path_provider/path_provider.dart';

const _arrival = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/arrival',
);

/// FolderAccess.swift keeps access to a receive folder outside Downloads.
const _folder = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/folder',
);

/// What "Share → ECOnnector" hands over (AppDelegate.swift's ShareInbox).
const _share = MethodChannel('io.github.serhanensar.ecosystemconnector/share');

/// Entry point of the macOS app. Screens come from ec_ui; this file only connects
/// the macOS-specific pieces.
void main() {
  // The share channel below needs the binding before runEcApp starts it.
  WidgetsFlutterBinding.ensureInitialized();
  final files = StreamController<List<String>>.broadcast();
  final items = StreamController<FlingContent>.broadcast();
  void received(Object? share) {
    final map = (share as Map?) ?? const {};
    if (map['files'] case final List<Object?> paths) {
      files.add(List<String>.from(paths));
    } else if (map['url'] case final String url) {
      items.add(FlingPage(url));
    } else if (map['text'] case final String text) {
      items.add(FlingText(text));
    }
  }

  _share.setMethodCallHandler((call) async {
    if (call.method == 'shared') received(call.arguments);
  });
  // Shares that launched the app, then later ones. Asked once the streams below have
  // listeners, so nothing is lost.
  Future<void> takeEarly() async {
    final early = await _share.invokeMethod<List<Object?>>('take') ?? const [];
    early.forEach(received);
  }

  runEcApp(
    EcPlatform.macos,
    PlatformHooks(
      // Before the core starts, so it can still save to the folder chosen last time.
      beforeStart: () => _folder.invokeMethod<void>('restore'),
      chooseFolder: () => _folder.invokeMethod<String>('choose'),
      folderReset: () => _folder.invokeMethod<void>('forget'),
      sharedFiles: () async* {
        unawaited(Future<void>.delayed(Duration.zero, takeEarly));
        yield* files.stream;
      }(),
      sharedItems: items.stream,
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
