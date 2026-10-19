import 'dart:async';

import 'package:flutter/material.dart';

import '../controller.dart';
import '../model.dart';
import '../theme.dart';

/// Picks files and sends them to [device], showing progress in a bottom sheet.
Future<void> pickAndSend(
  BuildContext context,
  EcController controller,
  EcDevice device,
) async {
  final paths = await controller.pickFiles();
  if (paths.isEmpty || !context.mounted) return;
  await sendWithProgress(context, controller, device, paths);
}

/// Sends [paths] to [device], showing progress in a bottom sheet.
Future<void> sendWithProgress(
  BuildContext context,
  EcController controller,
  EcDevice device,
  List<String> paths,
) {
  return showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (_) =>
        SendSheet(status: controller.send(device, paths), device: device),
  );
}

/// Links [path] with [device] and sends it, showing progress in a bottom sheet.
Future<void> sendLinkedWithProgress(
  BuildContext context,
  EcController controller,
  EcDevice device,
  String path,
) {
  return showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (_) => SendSheet(
      status: controller.sendLinkedFile(device, path),
      device: device,
      linked: true,
    ),
  );
}

/// Shows the progress of one send.
class SendSheet extends StatefulWidget {
  const SendSheet({
    super.key,
    required this.status,
    required this.device,
    this.linked = false,
  });

  final Stream<SendStatus> status;
  final EcDevice device;

  /// A linked file: kept up to date on both devices from now on.
  final bool linked;

  @override
  State<SendSheet> createState() => _SendSheetState();
}

class _SendSheetState extends State<SendSheet> {
  late final StreamSubscription<SendStatus> _sub;
  SendStatus _status = const SendProgress(sent: 0, total: 0, paths: 0);
  final _watch = Stopwatch()..start();

  @override
  void initState() {
    super.initState();
    _sub = widget.status.listen((s) {
      if (mounted) setState(() => _status = s);
    });
  }

  @override
  void dispose() {
    _sub.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final palette = EcPalette.of(context);
    final name = widget.device.name;
    final Widget body = switch (_status) {
      SendProgress(:final sent, :final total, :final paths, :final fraction) =>
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              total == 0 ? 'Preparing files for $name…' : 'Sending to $name',
              style: text.titleMedium,
            ),
            const SizedBox(height: 12),
            LinearProgressIndicator(
              value: total == 0 ? null : fraction,
              color: palette.ember,
              borderRadius: BorderRadius.circular(4),
            ),
            const SizedBox(height: 8),
            if (total > 0)
              Text(
                '${formatBytes(sent)} of ${formatBytes(total)} · '
                '${formatBytes(sent / (_watch.elapsedMicroseconds / 1e6 + 1e-6))}/s'
                '${paths > 1 ? ' · $paths connections' : ''}',
                style: text.bodySmall,
              ),
          ],
        ),
      SendRetrying(:final reason) => Text(
        'Connection lost ($reason). Reconnecting…',
        style: text.bodyMedium,
      ),
      SendDone(:final files, :final bytes, :final bytesPerSecond) => Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.check_circle, size: 40),
          const SizedBox(height: 8),
          Text(
            widget.linked
                ? 'Linked with $name'
                : 'Sent ${files == 1 ? '1 file' : '$files files'} to $name',
            style: text.titleMedium,
          ),
          if (widget.linked)
            Text(
              'Edits on either device now reach the other.',
              style: text.bodySmall,
            ),
          Text(
            '${formatBytes(bytes)} at ${formatBytes(bytesPerSecond)}/s',
            style: text.bodySmall,
          ),
          const SizedBox(height: 12),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Done'),
          ),
        ],
      ),
      SendFailed(:final reason) => Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.error_outline, size: 40),
          const SizedBox(height: 8),
          Text('Could not send to $name', style: text.titleMedium),
          Text(
            '$reason. Sending again resumes where it stopped.',
            style: text.bodySmall,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 12),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Close'),
          ),
        ],
      ),
    };
    return Padding(
      padding: const EdgeInsets.fromLTRB(24, 0, 24, 32),
      child: body,
    );
  }
}
