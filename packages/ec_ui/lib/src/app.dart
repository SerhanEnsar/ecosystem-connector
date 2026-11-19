import 'package:flutter/material.dart';

import 'arrival/arrival_layer.dart';
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
    this.arrivals,
  });

  /// The platform this build runs on.
  final EcPlatform platform;

  /// The running device.
  final EcController controller;

  /// Arrival cards to draw over the app, where the platform shows them in the window
  /// (Windows). Null where the platform shows them elsewhere or as notifications.
  final Stream<ArrivalEntry>? arrivals;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'ECOnnector',
      theme: ecLightTheme(),
      darkTheme: ecDarkTheme(),
      home: HomeScreen(controller: controller, platform: platform),
      builder: (context, child) {
        final stream = arrivals;
        if (stream == null) return child!;
        return ArrivalLayer(arrivals: stream, child: child!);
      },
    );
  }
}
