import 'package:flutter/material.dart';

import 'controller.dart';
import 'platform.dart';
import 'screens/home_screen.dart';
import 'theme.dart';

/// Root widget of every ECOnnector app.
class ECOnnectorApp extends StatelessWidget {
  const ECOnnectorApp({
    super.key,
    required this.platform,
    required this.controller,
  });

  /// The platform this build runs on.
  final EcPlatform platform;

  /// The running device.
  final EcController controller;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'ECOnnector',
      theme: ecLightTheme(),
      darkTheme: ecDarkTheme(),
      home: HomeScreen(controller: controller, platform: platform),
    );
  }
}
