import 'package:flutter/material.dart';

import '../model.dart';

/// Files shared from another app: choose the device to send them to.
class ShareBar extends StatelessWidget {
  const ShareBar({
    super.key,
    required this.count,
    required this.devices,
    required this.onSend,
    required this.onDismiss,
  });

  final int count;
  final List<EcDevice> devices;
  final ValueChanged<EcDevice> onSend;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final online = devices.where((d) => d.online).toList();
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 8, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    count == 1 ? 'Send 1 file to' : 'Send $count files to',
                    style: text.titleSmall,
                  ),
                ),
                IconButton(
                  tooltip: 'Cancel',
                  onPressed: onDismiss,
                  icon: const Icon(Icons.close),
                ),
              ],
            ),
            if (online.isEmpty)
              Text(
                'None of your paired devices is online.',
                style: text.bodySmall,
              )
            else
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final device in online)
                    ActionChip(
                      avatar: const Icon(Icons.north_east, size: 16),
                      label: Text(device.name),
                      onPressed: () => onSend(device),
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}
