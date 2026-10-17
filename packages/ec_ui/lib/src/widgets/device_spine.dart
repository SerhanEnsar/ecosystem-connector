import 'package:flutter/material.dart';

import '../model.dart';
import '../theme.dart';

/// This device at the top, paired devices hanging off one vertical line below it.
/// Online devices are filled nodes on a solid line; offline ones are hollow.
/// While [busy] is true the line carries a moving ember pulse.
class DeviceSpine extends StatelessWidget {
  const DeviceSpine({
    super.key,
    required this.myName,
    required this.myShortId,
    required this.devices,
    required this.busy,
    required this.onTap,
    required this.onForget,
  });

  final String myName;
  final String myShortId;
  final List<EcDevice> devices;
  final bool busy;
  final ValueChanged<EcDevice> onTap;
  final ValueChanged<EcDevice> onForget;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return ListView(
      padding: const EdgeInsets.fromLTRB(20, 8, 20, 120),
      children: [
        _Row(
          node: _Node.self,
          isLast: devices.isEmpty,
          busy: busy,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('This device', style: text.labelMedium),
              Text(
                myName,
                style: text.headlineSmall?.copyWith(
                  fontWeight: FontWeight.w700,
                ),
              ),
              Text(
                myShortId,
                style: ecMono.copyWith(fontSize: 13, letterSpacing: 1.2),
              ),
            ],
          ),
        ),
        for (final (i, device) in devices.indexed)
          _Row(
            node: device.online ? _Node.online : _Node.offline,
            isLast: i == devices.length - 1,
            busy: busy,
            child: _DeviceTile(
              device: device,
              onTap: () => onTap(device),
              onForget: () => onForget(device),
            ),
          ),
      ],
    );
  }
}

enum _Node { self, online, offline }

class _Row extends StatelessWidget {
  const _Row({
    required this.node,
    required this.isLast,
    required this.busy,
    required this.child,
  });

  final _Node node;
  final bool isLast;
  final bool busy;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            width: 36,
            child: _SpineSegment(node: node, isLast: isLast, busy: busy),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.symmetric(vertical: 14),
              child: child,
            ),
          ),
        ],
      ),
    );
  }
}

class _SpineSegment extends StatefulWidget {
  const _SpineSegment({
    required this.node,
    required this.isLast,
    required this.busy,
  });

  final _Node node;
  final bool isLast;
  final bool busy;

  @override
  State<_SpineSegment> createState() => _SpineSegmentState();
}

class _SpineSegmentState extends State<_SpineSegment>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulse = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 1400),
  );

  @override
  void initState() {
    super.initState();
    _sync();
  }

  @override
  void didUpdateWidget(_SpineSegment old) {
    super.didUpdateWidget(old);
    _sync();
  }

  void _sync() {
    final reduceMotion = WidgetsBinding
        .instance
        .platformDispatcher
        .accessibilityFeatures
        .disableAnimations;
    if (widget.busy && !reduceMotion) {
      if (!_pulse.isAnimating) _pulse.repeat();
    } else {
      _pulse.stop();
    }
  }

  @override
  void dispose() {
    _pulse.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final palette = EcPalette.of(context);
    return AnimatedBuilder(
      animation: _pulse,
      builder: (context, _) => CustomPaint(
        painter: _SpinePainter(
          node: widget.node,
          isLast: widget.isLast,
          link: palette.link,
          muted: palette.muted,
          ember: palette.ember,
          pulse: widget.busy ? _pulse.value : null,
          surface: Theme.of(context).colorScheme.surface,
        ),
      ),
    );
  }
}

class _SpinePainter extends CustomPainter {
  _SpinePainter({
    required this.node,
    required this.isLast,
    required this.link,
    required this.muted,
    required this.ember,
    required this.pulse,
    required this.surface,
  });

  final _Node node;
  final bool isLast;
  final Color link;
  final Color muted;
  final Color ember;
  final double? pulse;
  final Color surface;

  static const _nodeY = 30.0;

  @override
  void paint(Canvas canvas, Size size) {
    final x = size.width / 2;
    final line = Paint()
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.round
      ..color = node == _Node.offline ? muted : link;

    if (node != _Node.self) {
      canvas.drawLine(Offset(x, 0), Offset(x, _nodeY), line);
    }
    if (!isLast) {
      canvas.drawLine(
        Offset(x, _nodeY),
        Offset(x, size.height),
        line..color = link,
      );
    }
    if (pulse != null && (!isLast || node != _Node.self)) {
      final y = pulse! * size.height;
      canvas.drawCircle(Offset(x, y), 3.5, Paint()..color = ember);
    }

    final radius = node == _Node.self ? 9.0 : 7.0;
    canvas.drawCircle(Offset(x, _nodeY), radius + 3, Paint()..color = surface);
    switch (node) {
      case _Node.self:
      case _Node.online:
        canvas.drawCircle(Offset(x, _nodeY), radius, Paint()..color = link);
      case _Node.offline:
        canvas.drawCircle(
          Offset(x, _nodeY),
          radius - 1,
          Paint()
            ..style = PaintingStyle.stroke
            ..strokeWidth = 2
            ..color = muted,
        );
    }
  }

  @override
  bool shouldRepaint(_SpinePainter old) =>
      old.pulse != pulse ||
      old.node != node ||
      old.isLast != isLast ||
      old.link != link ||
      old.surface != surface;
}

class _DeviceTile extends StatelessWidget {
  const _DeviceTile({
    required this.device,
    required this.onTap,
    required this.onForget,
  });

  final EcDevice device;
  final VoidCallback onTap;
  final VoidCallback onForget;

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    final status = device.online ? 'Online' : 'Offline';
    return Semantics(
      button: device.online,
      label: '${device.name}, $status',
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: device.online ? onTap : null,
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(device.name, style: text.titleMedium),
                  Text.rich(
                    TextSpan(
                      children: [
                        TextSpan(text: '$status · '),
                        TextSpan(text: device.shortId, style: ecMono),
                      ],
                    ),
                    style: text.bodySmall,
                  ),
                ],
              ),
            ),
            if (device.online)
              const Padding(
                padding: EdgeInsets.only(right: 4),
                child: Icon(Icons.north_east, size: 20),
              ),
            PopupMenuButton<void>(
              tooltip: 'More',
              itemBuilder: (_) => [
                PopupMenuItem(
                  onTap: onForget,
                  child: const Text('Forget this device'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
