import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:path_provider/path_provider.dart';

/// Entry point of the macOS app. Screens come from ec_ui; this file only connects
/// the macOS-specific pieces.
void main() {
  runEcApp(
    EcPlatform.macos,
    PlatformHooks(
      opensFlings: true,
      deviceName: () async => Platform.localHostname.split('.').first,
      receiveDir: () async {
        final downloads = await getDownloadsDirectory();
        final dir = Directory(
          '${downloads!.path}${Platform.pathSeparator}Ecosystem Connector',
        );
        await dir.create(recursive: true);
        return dir.path;
      },
    ),
  );
}
