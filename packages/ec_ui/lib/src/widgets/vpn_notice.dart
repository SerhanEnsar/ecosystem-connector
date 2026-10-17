import 'package:flutter/material.dart';

/// Explains that a VPN can stop other devices from reaching this one.
class VpnNotice extends StatelessWidget {
  const VpnNotice({super.key, required this.onDismiss});

  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 12),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.vpn_lock_outlined),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('A VPN is on', style: text.titleSmall),
                  const SizedBox(height: 2),
                  Text(
                    "If your other devices can't reach this one, exclude "
                    'Ecosystem Connector from the VPN in its settings.',
                    style: text.bodySmall,
                  ),
                ],
              ),
            ),
            IconButton(
              tooltip: 'Dismiss',
              onPressed: onDismiss,
              icon: const Icon(Icons.close),
            ),
          ],
        ),
      ),
    );
  }
}
