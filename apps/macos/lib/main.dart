import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';

/// Entry point of the macOS app.
///
/// Platform-specific integrations live in this app's native folder (macos/); the
/// screens come from the shared ec_ui package.
void main() {
  runApp(const EcosystemConnectorApp(platform: EcPlatform.macos));
}
