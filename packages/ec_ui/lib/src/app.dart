import 'package:flutter/material.dart';

import 'platform.dart';
import 'screens/devices_screen.dart';
import 'theme.dart';

/// Root widget of every Ecosystem Connector app.
class EcosystemConnectorApp extends StatelessWidget {
  const EcosystemConnectorApp({super.key, required this.platform});

  /// The platform this build runs on.
  final EcPlatform platform;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Ecosystem Connector',
      theme: ecLightTheme(),
      darkTheme: ecDarkTheme(),
      home: DevicesScreen(platform: platform),
    );
  }
}
