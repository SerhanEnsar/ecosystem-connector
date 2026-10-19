import 'package:flutter/material.dart';

import '../model.dart';

/// Something shared from another app: choose the device to send it to.
class ShareBar extends StatelessWidget {
  const ShareBar({
    super.key,
    required this.title,
    required this.devices,
    required this.onSend,
    required this.onDismiss,
    this.linkedOffered = false,
    this.linked = false,
    this.onLinkedChanged,
  });

  /// What to send, such as "Send 2 files to".
  final String title;
  final List<EcDevice> devices;
  final ValueChanged<EcDevice> onSend;
  final VoidCallback onDismiss;

  /// Whether "Send linked" can be chosen (a single file, and a device that can keep
  /// linked files is online).
  final bool linkedOffered;

  /// Whether "Send linked" is chosen; then only devices that support it can be picked.
  final bool linked;
  final ValueChanged<bool>? onLinkedChanged;

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
                    title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
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
            if (linkedOffered && online.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Wrap(
                  spacing: 8,
                  children: [
                    ChoiceChip(
                      label: const Text('Send'),
                      selected: !linked,
                      onSelected: (_) => onLinkedChanged?.call(false),
                    ),
                    ChoiceChip(
                      avatar: const Icon(Icons.link, size: 16),
                      label: const Text('Send linked'),
                      selected: linked,
                      onSelected: (_) => onLinkedChanged?.call(true),
                    ),
                  ],
                ),
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
                      tooltip: linked && !device.linkedFiles
                          ? '${device.name} needs an update to keep linked files'
                          : null,
                      onPressed: linked && !device.linkedFiles
                          ? null
                          : () => onSend(device),
                    ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}
