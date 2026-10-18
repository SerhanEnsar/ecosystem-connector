import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../controller.dart';
import '../model.dart';
import '../widgets/code_display.dart';

/// Opens the pairing flow in a bottom sheet.
Future<void> showPairingSheet(
  BuildContext context,
  EcController controller, {
  String? invite,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    showDragHandle: true,
    builder: (_) => PairingSheet(controller: controller, invite: invite),
  );
}

/// Pair by showing a code (QR + link) or by scanning / pasting another device's.
class PairingSheet extends StatefulWidget {
  const PairingSheet({super.key, required this.controller, this.invite});

  final EcController controller;

  /// A pairing link to join right away (the app was opened with it).
  final String? invite;

  @override
  State<PairingSheet> createState() => _PairingSheetState();
}

class _PairingSheetState extends State<PairingSheet> {
  StreamSubscription<PairingStep>? _sub;
  PairingStep? _step;
  String? _invite;
  bool _answered = false;
  final _link = TextEditingController();

  EcController get _c => widget.controller;

  @override
  void initState() {
    super.initState();
    final invite = widget.invite;
    if (invite != null) {
      WidgetsBinding.instance.addPostFrameCallback(
        (_) => _listen(_c.joinPairing(invite)),
      );
    }
  }

  void _listen(Stream<PairingStep> steps) {
    _sub?.cancel();
    setState(() {
      _step = null;
      _invite = null;
      _answered = false;
    });
    _sub = steps.listen((step) {
      if (!mounted) return;
      setState(() {
        if (step is PairingInvite) {
          _invite = step.uri;
        } else {
          _step = step;
        }
      });
    });
  }

  Future<void> _scan() async {
    final invite = await _c.scanInvite();
    if (invite != null) _listen(_c.joinPairing(invite));
  }

  void _answer(bool accept) {
    setState(() => _answered = true);
    _c.answerPairing(accept);
  }

  @override
  void dispose() {
    _sub?.cancel();
    _link.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final step = _step;
    final Widget body = switch (step) {
      PairingConfirm() => _confirm(step, text),
      PairingDone(:final name) => _result(
        icon: Icons.check_circle,
        title: 'Paired with $name',
        detail: 'You can now send files to each other.',
      ),
      PairingFailed(:final reason) => _result(
        icon: Icons.error_outline,
        title: 'Pairing did not complete',
        detail: '$reason. Start again on both devices.',
      ),
      _ when _invite != null => _hosting(_invite!, text),
      _ => _choose(text),
    };
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        0,
        24,
        24 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: AnimatedSize(
        duration: const Duration(milliseconds: 200),
        child: body,
      ),
    );
  }

  Widget _choose(TextTheme text) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text('Pair a device', style: text.titleLarge),
        const SizedBox(height: 4),
        Text(
          'Show a code on one device and scan it with the other. Both must be on the same network.',
          style: text.bodyMedium,
        ),
        const SizedBox(height: 20),
        if (_c.canScan) ...[
          FilledButton.icon(
            onPressed: _scan,
            icon: const Icon(Icons.qr_code_scanner),
            label: const Text('Scan a code'),
          ),
          const SizedBox(height: 8),
        ],
        OutlinedButton.icon(
          onPressed: () => _listen(_c.hostPairing()),
          icon: const Icon(Icons.qr_code_2),
          label: const Text('Show a code'),
        ),
        const SizedBox(height: 20),
        TextField(
          controller: _link,
          decoration: InputDecoration(
            labelText: 'Or paste a pairing link',
            hintText: 'ecpair:1?…',
            suffixIcon: IconButton(
              tooltip: 'Pair',
              icon: const Icon(Icons.arrow_forward),
              onPressed: () {
                final link = _link.text.trim();
                if (link.isNotEmpty) _listen(_c.joinPairing(link));
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _hosting(String uri, TextTheme text) {
    final scheme = Theme.of(context).colorScheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text('Scan this code on your other device', style: text.titleMedium),
        const SizedBox(height: 16),
        Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Colors.white,
            borderRadius: BorderRadius.circular(16),
          ),
          child: QrImageView(
            data: uri,
            size: 220,
            backgroundColor: Colors.white,
          ),
        ),
        const SizedBox(height: 12),
        TextButton.icon(
          onPressed: () => Clipboard.setData(ClipboardData(text: uri)),
          icon: const Icon(Icons.copy, size: 18),
          label: const Text('Copy pairing link'),
        ),
        const SizedBox(height: 4),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            SizedBox.square(
              dimension: 14,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: scheme.primary,
              ),
            ),
            const SizedBox(width: 8),
            Flexible(
              child: Text(
                'Waiting for the other device · works once, for 2 minutes',
                style: text.bodySmall,
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _confirm(PairingConfirm step, TextTheme text) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(
          'Does ${step.peerName} show this code?',
          style: text.titleMedium,
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 16),
        CodeDisplay(step.code),
        const SizedBox(height: 20),
        if (_answered)
          Text('Waiting for the other device…', style: text.bodyMedium)
        else
          Row(
            children: [
              Expanded(
                child: OutlinedButton(
                  onPressed: () => _answer(false),
                  child: const Text("Codes don't match"),
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: FilledButton(
                  onPressed: () => _answer(true),
                  child: const Text('Codes match'),
                ),
              ),
            ],
          ),
      ],
    );
  }

  Widget _result({
    required IconData icon,
    required String title,
    required String detail,
  }) {
    final text = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 40),
        const SizedBox(height: 12),
        Text(title, style: text.titleMedium),
        const SizedBox(height: 4),
        Text(detail, style: text.bodyMedium, textAlign: TextAlign.center),
        const SizedBox(height: 16),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Done'),
        ),
      ],
    );
  }
}
