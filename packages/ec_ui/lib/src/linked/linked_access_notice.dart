import 'package:flutter/material.dart';

import '../controller.dart';

/// Explains why linked files need a permission (Android: all files access) and
/// opens the system page where it is given. The rest of the app works without it.
class LinkedAccessNotice extends StatelessWidget {
  const LinkedAccessNotice({
    super.key,
    required this.controller,
    this.onDismiss,
  });

  final EcController controller;

  /// Hides the notice; null where it stays until the permission is given.
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Card(
      margin: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 8, 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Icon(Icons.folder_shared_outlined),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Linked files need all files access',
                    style: text.titleSmall,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    'Sending and receiving work without it. But when another app, '
                    'such as Samsung Notes, saves a linked file, Android makes the '
                    'file that app\'s, and only with this permission can ECOnnector '
                    'still keep it up to date. ECOnnector changes only the files '
                    'you link.',
                    style: text.bodySmall,
                  ),
                  Align(
                    alignment: AlignmentDirectional.centerEnd,
                    child: TextButton(
                      onPressed: controller.openLinkedAccessSettings,
                      child: const Text('Allow in settings'),
                    ),
                  ),
                ],
              ),
            ),
            if (onDismiss case final dismiss?)
              IconButton(
                tooltip: 'Not now',
                onPressed: dismiss,
                icon: const Icon(Icons.close),
              ),
          ],
        ),
      ),
    );
  }
}
