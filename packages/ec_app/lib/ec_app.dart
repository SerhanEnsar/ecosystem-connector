/// Starts an Ecosystem Connector app: the Rust core, the shared UI, and the
/// platform hooks each app supplies.
library;

import 'package:ec_bridge/ec_bridge.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

import 'src/bridge_controller.dart';
import 'src/hooks.dart';
import 'src/notifier.dart';

export 'src/hooks.dart';

/// Starts the core and shows the app. Errors during start-up are shown on screen.
Future<void> runEcApp(EcPlatform platform, PlatformHooks hooks) async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    await RustLib.init();
    await hooks.beforeStart?.call();
    final support = await getApplicationSupportDirectory();
    final core = await EcCore.start(
      config: CoreConfig(
        home: '${support.path}/core',
        deviceName: await hooks.deviceName(),
        receiveDir: await hooks.receiveDir(),
      ),
    );
    final notifier = Notifier();
    await notifier.init();
    runApp(
      EcosystemConnectorApp(
        platform: platform,
        controller: BridgeController(core, hooks, notifier),
      ),
    );
  } catch (error) {
    runApp(_StartupError(error));
  }
}

class _StartupError extends StatelessWidget {
  const _StartupError(this.error);

  final Object error;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      theme: ecLightTheme(),
      darkTheme: ecDarkTheme(),
      home: Scaffold(
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.error_outline, size: 40),
                const SizedBox(height: 12),
                const Text('Ecosystem Connector could not start'),
                const SizedBox(height: 8),
                SelectableText('$error', textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
