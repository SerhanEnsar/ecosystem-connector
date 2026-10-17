import 'package:flutter/material.dart';

import '../model.dart';
import '../theme.dart';

/// Shows an incoming transfer at the bottom of the home screen.
class ReceiveBanner extends StatelessWidget {
  const ReceiveBanner({super.key, required this.status});

  final ReceiveStatus status;

  @override
  Widget build(BuildContext context) {
    final palette = EcPalette.of(context);
    final text = Theme.of(context).textTheme;
    final files = status.files == 1 ? '1 file' : '${status.files} files';
    return Card(
      margin: const EdgeInsets.all(16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              'Receiving $files from ${status.from}',
              style: text.titleSmall,
            ),
            const SizedBox(height: 10),
            LinearProgressIndicator(
              value: status.fraction,
              color: palette.ember,
              borderRadius: BorderRadius.circular(4),
            ),
            const SizedBox(height: 6),
            Text(
              '${formatBytes(status.received)} of ${formatBytes(status.total)}',
              style: text.bodySmall,
            ),
          ],
        ),
      ),
    );
  }
}
