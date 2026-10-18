import 'package:flutter/material.dart';

import '../controller.dart';
import '../fling/fling_setup_card.dart';

/// Opens the settings page.
Future<void> showSettings(BuildContext context, EcController controller) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => SettingsScreen(controller: controller),
      ),
    );

/// Where received files go, and (on Android) flinging from the screen edge.
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key, required this.controller});

  final EcController controller;

  Future<void> _choose(BuildContext context) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      await controller.chooseReceiveDir();
    } on Object catch (e) {
      messenger.showSnackBar(
        SnackBar(content: Text('That folder cannot be used: $e')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListenableBuilder(
        listenable: controller,
        builder: (context, _) {
          final setup = controller.flingSetup;
          return ListView(
            padding: const EdgeInsets.only(bottom: 24),
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 16, 16, 4),
                child: Text('Received files', style: text.titleSmall),
              ),
              ListTile(
                leading: const Icon(Icons.folder_outlined),
                title: SelectableText(
                  controller.receiveDir,
                  key: const Key('receive-dir'),
                ),
                subtitle: Text(
                  controller.canChooseReceiveDir
                      ? (controller.receiveDirIsDefault
                            ? 'The default folder. New files are saved here.'
                            : 'New files are saved here.')
                      : 'New files are saved here. After a file arrives, its card '
                            'can show it in this folder.',
                ),
              ),
              if (controller.canChooseReceiveDir)
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Wrap(
                    spacing: 8,
                    children: [
                      FilledButton.tonal(
                        onPressed: () => _choose(context),
                        child: const Text('Change folder…'),
                      ),
                      if (!controller.receiveDirIsDefault)
                        TextButton(
                          onPressed: controller.useDefaultReceiveDir,
                          child: const Text('Use the default folder'),
                        ),
                    ],
                  ),
                ),
              if (setup != null) ...[
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 24, 16, 0),
                  child: Text('Fling', style: text.titleSmall),
                ),
                FlingSetupCard(controller: controller, setup: setup),
              ],
            ],
          );
        },
      ),
    );
  }
}
