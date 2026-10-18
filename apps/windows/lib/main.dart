import 'dart:io';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:path_provider/path_provider.dart';

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
    ),
  );
}
