import 'package:flutter/material.dart';

void main() {
  runApp(const EcosystemConnectorApp());
}

/// Root widget of the Ecosystem Connector app.
///
/// The device list, pairing and transfer screens are added in later phases;
/// see docs/ARCHITECTURE.md in the repository root.
class EcosystemConnectorApp extends StatelessWidget {
  const EcosystemConnectorApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Ecosystem Connector',
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF2F6FED),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorSchemeSeed: const Color(0xFF2F6FED),
        brightness: Brightness.dark,
        useMaterial3: true,
      ),
      home: const DevicesScreen(),
    );
  }
}

/// Lists paired and nearby devices. Empty until discovery lands.
class DevicesScreen extends StatelessWidget {
  const DevicesScreen({super.key});

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
          ],
        ),
      ),
    );
  }
}
