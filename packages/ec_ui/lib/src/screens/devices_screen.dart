import 'package:flutter/material.dart';

import '../platform.dart';

/// Lists paired and nearby devices. Empty until the Rust core is connected
/// (phase 3).
class DevicesScreen extends StatelessWidget {
  const DevicesScreen({super.key, required this.platform});

  /// The platform this build runs on.
  final EcPlatform platform;

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Ecosystem Connector')),
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.devices_other, size: 64),
            const SizedBox(height: 16),
            Text('No devices yet', style: textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(
              'Paired devices on your network will appear here.',
              style: textTheme.bodyMedium,
            ),
            const SizedBox(height: 24),
            Text('${platform.label} build', style: textTheme.labelSmall),
          ],
        ),
      ),
    );
  }
}
