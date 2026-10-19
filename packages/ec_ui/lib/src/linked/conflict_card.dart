import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../arrival/arrival_card.dart';
import '../controller.dart';
import '../model.dart';

/// A linked file changed on two devices: both versions side by side, and the choice.
///
/// It looks like the arrival card but does not leave on its own; closing it keeps the
/// conflict, which stays in Settings → Linked files until the user chooses.
class ConflictCard extends StatelessWidget {
  const ConflictCard({
    super.key,
    required this.controller,
    required this.file,
    this.now,
  });

  final EcController controller;
  final EcLinkedFile file;

  /// For tests: the current time.
  final DateTime? now;

  /// The widest the card gets; on narrow screens it fits the screen.
  static const maxWidth = 400.0;

  @override
  Widget build(BuildContext context) {
    final conflict = file.conflict;
    if (conflict == null) return const SizedBox.shrink();
    const ink = Colors.white;
    final quiet = TextStyle(color: ink.withValues(alpha: 0.75), fontSize: 12);
    String line(String device, DateTime at, int size) =>
        '$device · ${formatWhen(at, now: now)} · ${formatBytes(size)}';

    return Align(
      alignment: Alignment.topCenter,
      child: Padding(
        padding: const EdgeInsets.only(top: 12),
        child: FrostedGlass(
          width: math.min(maxWidth, MediaQuery.sizeOf(context).width - 32),
          height: 184,
          radius: 22,
          shadow: 1,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 12, 12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    const Icon(Icons.call_split, size: 18, color: ink),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        '${file.name} changed on two devices',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(
                          color: ink,
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    Semantics(
                      button: true,
                      label: 'Close',
                      child: InkResponse(
                        onTap: () => controller.dismissConflict(file.id),
                        radius: 18,
                        child: const Icon(Icons.close, size: 16, color: ink),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                Text(
                  line(conflict.mineDevice, conflict.mineAt, conflict.mineSize),
                  style: quiet,
                ),
                Text(
                  line(
                    conflict.theirsDevice,
                    conflict.theirsAt,
                    conflict.theirsSize,
                  ),
                  style: quiet,
                ),
                const Spacer(),
                // Long device names or a narrow screen put buttons on a second line.
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    FrostedPill(
                      "Keep ${conflict.mineDevice}'s",
                      onTap: () => controller.resolveConflict(
                        file.id,
                        keepTheirs: false,
                      ),
                    ),
                    FrostedPill(
                      "Keep ${conflict.theirsDevice}'s",
                      onTap: () =>
                          controller.resolveConflict(file.id, keepTheirs: true),
                    ),
                    FrostedPill(
                      'Open both',
                      quiet: true,
                      onTap: () {
                        controller.open(file.path);
                        controller.open(conflict.theirsPath);
                      },
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
