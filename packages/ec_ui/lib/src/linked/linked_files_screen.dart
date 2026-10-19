import 'package:flutter/material.dart';

import '../controller.dart';
import '../model.dart';
import '../theme.dart';
import 'conflict_card.dart';

/// Opens the list of linked files.
Future<void> showLinkedFiles(BuildContext context, EcController controller) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => LinkedFilesScreen(controller: controller),
      ),
    );

/// What a linked file's state reads as.
String linkedStateText(EcLinkedFile file) => switch (file.state) {
  EcLinkedState.current => 'Up to date',
  EcLinkedState.updating => 'Updating…',
  EcLinkedState.conflict =>
    'Conflict (${formatBytes(file.conflict?.theirsSize ?? 0)} waiting)',
  EcLinkedState.missing => 'Missing',
};

/// Every linked file: its devices, its state, its log, and what can be done with it.
class LinkedFilesScreen extends StatelessWidget {
  const LinkedFilesScreen({super.key, required this.controller});

  final EcController controller;

  Future<void> _link(BuildContext context) async {
    final devices = controller.devices
        .where((d) => d.online && d.linkedFiles)
        .toList();
    if (devices.isEmpty) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text(
            'No device that can keep linked files is online. '
            'It may need an update.',
          ),
        ),
      );
      return;
    }
    final device = devices.length == 1
        ? devices.single
        : await showDialog<EcDevice>(
            context: context,
            builder: (context) => SimpleDialog(
              title: const Text('Link a file with'),
              children: [
                for (final device in devices)
                  SimpleDialogOption(
                    onPressed: () => Navigator.pop(context, device),
                    child: Text(device.name),
                  ),
              ],
            ),
          );
    if (device != null) await controller.linkFile(device);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Linked files'),
        actions: [
          TextButton.icon(
            onPressed: () => _link(context),
            icon: const Icon(Icons.add_link),
            label: const Text('Link a file'),
          ),
        ],
      ),
      body: ListenableBuilder(
        listenable: controller,
        builder: (context, _) {
          final files = controller.linkedFiles;
          if (files.isEmpty) {
            return const Center(
              child: Padding(
                padding: EdgeInsets.all(32),
                child: Text(
                  'No linked files yet. Choose "Send linked" when you send a file, '
                  'and edits on either device reach the other.',
                  textAlign: TextAlign.center,
                ),
              ),
            );
          }
          return ListView(
            padding: const EdgeInsets.only(bottom: 24),
            children: [
              for (final file in files)
                _LinkedTile(controller: controller, file: file),
            ],
          );
        },
      ),
    );
  }
}

class _LinkedTile extends StatelessWidget {
  const _LinkedTile({required this.controller, required this.file});

  final EcController controller;
  final EcLinkedFile file;

  Future<void> _leave(BuildContext context) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Leave the link of ${file.name}?'),
        content: const Text(
          'The file stays on this device but is no longer kept up to date. '
          'The other devices keep their copies.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Leave link'),
          ),
        ],
      ),
    );
    if (confirmed ?? false) await controller.leaveLink(file.id);
  }

  Future<void> _resolve(BuildContext context) => showDialog<void>(
    context: context,
    builder: (context) => Dialog(
      backgroundColor: Colors.transparent,
      child: ListenableBuilder(
        listenable: controller,
        builder: (context, _) {
          final current = controller.linkedFiles
              .where((f) => f.id == file.id)
              .firstOrNull;
          if (current == null || current.state != EcLinkedState.conflict) {
            WidgetsBinding.instance.addPostFrameCallback(
              (_) => Navigator.of(context).maybePop(),
            );
          }
          return ConflictCard(controller: controller, file: current ?? file);
        },
      ),
    ),
  );

  @override
  Widget build(BuildContext context) {
    final palette = EcPalette.of(context);
    final text = Theme.of(context).textTheme;
    final warn =
        file.state == EcLinkedState.conflict ||
        file.state == EcLinkedState.missing;
    return ExpansionTile(
      key: ValueKey(file.id),
      leading: Icon(switch (file.state) {
        EcLinkedState.current => Icons.link,
        EcLinkedState.updating => Icons.sync,
        EcLinkedState.conflict => Icons.call_split,
        EcLinkedState.missing => Icons.link_off,
      }, color: warn ? palette.ember : null),
      title: Text(file.name, maxLines: 1, overflow: TextOverflow.ellipsis),
      subtitle: Text(
        '${linkedStateText(file)} · ${file.devices.join(', ')}',
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
      expandedCrossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SelectableText(file.path, style: text.bodySmall),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: [
            if (file.state == EcLinkedState.conflict) ...[
              FilledButton.tonal(
                onPressed: () => _resolve(context),
                child: const Text('Resolve'),
              ),
              TextButton(
                onPressed: () => controller.deleteConflict(file.id),
                child: const Text('Delete waiting version'),
              ),
            ],
            if (file.state == EcLinkedState.missing)
              FilledButton.tonal(
                onPressed: () => controller.locateLinked(file.id),
                child: const Text('Show where it is'),
              ),
            TextButton(
              onPressed: () => _leave(context),
              child: const Text('Leave link'),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Text('Log', style: text.titleSmall),
        FutureBuilder<List<EcLinkedLog>>(
          future: controller.linkedLog(file.id),
          builder: (context, snapshot) {
            final log = snapshot.data ?? const <EcLinkedLog>[];
            if (log.isEmpty) {
              return Text('Nothing yet.', style: text.bodySmall);
            }
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final entry in log.take(20))
                  Padding(
                    padding: const EdgeInsets.only(top: 4),
                    child: Text(
                      '${formatWhen(entry.at)} · ${entry.device} ${entry.text}',
                      style: text.bodySmall,
                    ),
                  ),
              ],
            );
          },
        ),
      ],
    );
  }
}
