import 'package:flutter/material.dart';

import '../theme.dart';

/// The six-digit pairing code, large and evenly spaced like a hardware display.
class CodeDisplay extends StatelessWidget {
  const CodeDisplay(this.code, {super.key});

  final String code;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      label: 'Code ${code.split('').join(' ')}',
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 16),
        decoration: BoxDecoration(
          color: scheme.onSurface,
          borderRadius: BorderRadius.circular(14),
        ),
        child: Text(
          code,
          style: ecMono.copyWith(
            fontSize: 40,
            fontWeight: FontWeight.w600,
            letterSpacing: 6,
            color: scheme.surface,
          ),
        ),
      ),
    );
  }
}
