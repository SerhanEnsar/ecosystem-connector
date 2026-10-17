import 'package:flutter/material.dart';

import '../controller.dart';
import '../model.dart';

/// Explains flinging and walks through what it needs, one switch or button each.
class FlingSetupCard extends StatelessWidget {
  const FlingSetupCard({
    super.key,
    required this.controller,
    required this.setup,
    this.onDismiss,
  });

  final EcController controller;
  final FlingSetup setup;
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                const Icon(Icons.swipe_right_alt),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Fling from the edge of the screen',
                    style: text.titleSmall,
                  ),
                ),
                if (onDismiss != null)
                  IconButton(
                    tooltip: 'Hide',
                    onPressed: onDismiss,
                    icon: const Icon(Icons.close),
                  ),
              ],
            ),
            Padding(
              padding: const EdgeInsets.only(left: 36, right: 8),
              child: Text(
                'Hold the handle on the left edge, then drag what is on screen '
                'onto one of your devices. It opens there.',
                style: text.bodySmall,
              ),
            ),
            const SizedBox(height: 8),
            _Step(
              done: setup.canDrawOverlays,
              label: 'Allow showing the handle over other apps',
              action: 'Allow',
              onPressed: controller.openOverlaySettings,
            ),
            _Step(
              done: setup.screenAccess,
              label: 'Turn on screen access for screenshots and web pages',
              action: 'Open settings',
              onPressed: controller.openScreenAccessSettings,
            ),
            SwitchListTile(
              contentPadding: const EdgeInsets.only(left: 36, right: 8),
              title: const Text('Show the handle'),
              value: setup.handleOn,
              onChanged: setup.canDrawOverlays
                  ? controller.setFlingHandle
                  : null,
            ),
          ],
        ),
      ),
    );
  }
}

class _Step extends StatelessWidget {
  const _Step({
    required this.done,
    required this.label,
    required this.action,
    required this.onPressed,
  });

  final bool done;
  final String label;
  final String action;
  final Future<void> Function() onPressed;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          Icon(
            done ? Icons.check_circle : Icons.radio_button_unchecked,
            size: 20,
            color: done ? Theme.of(context).colorScheme.primary : null,
          ),
          const SizedBox(width: 16),
          Expanded(child: Text(label)),
          if (!done) TextButton(onPressed: onPressed, child: Text(action)),
        ],
      ),
    );
  }
}
