import 'dart:io';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../controller.dart';
import '../model.dart';
import '../theme.dart';

/// What the finger is doing, fed by the platform (the overlay itself takes no
/// touches: on Android the edge handle keeps receiving the gesture that started on it
/// and forwards the positions here).
class FlingGesture extends ChangeNotifier {
  FlingContent? _content;
  Offset? _position;
  Offset? _released;

  /// What is being flung, once the hold started.
  FlingContent? get content => _content;

  /// Where the finger is, in logical pixels.
  Offset? get position => _position;

  /// Where the finger was lifted, once it was.
  Offset? get released => _released;

  /// The hold started at [at] with [content].
  void start(FlingContent content, Offset at) {
    _content = content;
    _position = at;
    _released = null;
    notifyListeners();
  }

  /// The finger moved.
  void move(Offset to) {
    if (_content == null || _released != null) return;
    _position = to;
    notifyListeners();
  }

  /// The finger was lifted at [at].
  void release(Offset at) {
    if (_content == null || _released != null) return;
    _position = at;
    _released = at;
    notifyListeners();
  }
}

/// How the fling ended.
enum FlingResult { sent, cancelled, failed }

/// Full-screen layer shown while flinging: what is being sent at the top, online
/// devices as targets on the right, and a card under the finger. Dropping the card on
/// a device sends it; letting go anywhere else cancels.
class FlingOverlay extends StatefulWidget {
  const FlingOverlay({
    super.key,
    required this.controller,
    required this.gesture,
    required this.onFinished,
  });

  final EcController controller;
  final FlingGesture gesture;

  /// Called once the closing animation is over.
  final ValueChanged<FlingResult> onFinished;

  /// Distance from a target's centre within which the card counts as over it.
  static const hitRadius = 64.0;

  /// How far left of a target the card docks while hovering it.
  static const dockGap = 150.0;

  /// Centres of the targets for [count] devices on a screen of [size]: a column near
  /// the right edge, centred vertically.
  static List<Offset> targetCentres(Size size, int count) {
    const spacing = 112.0;
    final x = size.width - 72;
    final top = size.height / 2 - (count - 1) * spacing / 2;
    return [for (var i = 0; i < count; i++) Offset(x, top + i * spacing)];
  }

  @override
  State<FlingOverlay> createState() => _FlingOverlayState();
}

enum _Phase { dragging, flying, returning, failed }

class _FlingOverlayState extends State<FlingOverlay>
    with SingleTickerProviderStateMixin {
  late final AnimationController _anim = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 420),
  );
  var _phase = _Phase.dragging;
  Offset? _from;
  Offset? _to;
  int? _hovered;
  String? _error;
  Size _size = Size.zero;

  List<EcDevice> get _online =>
      widget.controller.devices.where((d) => d.online).toList();

  @override
  void initState() {
    super.initState();
    widget.gesture.addListener(_onGesture);
  }

  @override
  void dispose() {
    widget.gesture.removeListener(_onGesture);
    _anim.dispose();
    super.dispose();
  }

  int? _targetAt(Offset point) {
    final centres = FlingOverlay.targetCentres(_size, _online.length);
    for (var i = 0; i < centres.length; i++) {
      if ((centres[i] - point).distance <= FlingOverlay.hitRadius) return i;
    }
    return null;
  }

  void _onGesture() {
    if (_phase != _Phase.dragging) return;
    final position = widget.gesture.position;
    if (position == null) return;
    final released = widget.gesture.released;
    if (released == null) {
      setState(() => _hovered = _targetAt(position));
      return;
    }
    final target = _targetAt(released);
    if (target == null) {
      _returnHome(released);
    } else {
      _flyTo(target, released);
    }
  }

  Future<void> _returnHome(Offset from) async {
    setState(() {
      _phase = _Phase.returning;
      _from = from;
      _to = Offset(-80, from.dy);
    });
    await _anim.forward(from: 0);
    widget.onFinished(FlingResult.cancelled);
  }

  Future<void> _flyTo(int index, Offset from) async {
    final device = _online[index];
    final centre = FlingOverlay.targetCentres(_size, _online.length)[index];
    setState(() {
      _phase = _Phase.flying;
      _hovered = index;
      _from = from;
      _to = centre;
    });
    final content = widget.gesture.content!;
    final first = widget.controller
        .fling(device, content)
        .firstWhere((s) => s is! SendProgress || s.sent > 0 || s.total == 0)
        .timeout(
          const Duration(seconds: 8),
          onTimeout: () => const SendRetrying('slow'),
        );
    await _anim.forward(from: 0);
    final status = await first.catchError((Object e) => SendFailed('$e'));
    if (status is SendFailed) {
      setState(() {
        _phase = _Phase.failed;
        _error = 'Could not send to ${device.name}: ${status.reason}';
      });
      await Future<void>.delayed(const Duration(seconds: 2));
      widget.onFinished(FlingResult.failed);
      return;
    }
    // Large files keep going in the background; the overlay can close.
    await Future<void>.delayed(const Duration(milliseconds: 250));
    widget.onFinished(FlingResult.sent);
  }

  @override
  Widget build(BuildContext context) {
    final palette = EcPalette.of(context);
    final scheme = Theme.of(context).colorScheme;
    final text = Theme.of(context).textTheme;
    final content = widget.gesture.content;
    final reduceMotion = MediaQuery.of(context).disableAnimations;
    if (reduceMotion) _anim.duration = const Duration(milliseconds: 1);

    return LayoutBuilder(
      builder: (context, constraints) {
        _size = constraints.biggest;
        final devices = _online;
        final centres = FlingOverlay.targetCentres(_size, devices.length);
        return AnimatedBuilder(
          animation: _anim,
          builder: (context, _) {
            final t = Curves.easeInCubic.transform(_anim.value);
            final fading = _phase == _Phase.returning ? 1 - t : 1.0;
            final cardAt = switch (_phase) {
              // Over a target, the card docks beside it so the device stays visible.
              _Phase.dragging when _hovered != null =>
                centres[_hovered!] - const Offset(FlingOverlay.dockGap, 0),
              _Phase.dragging => widget.gesture.position,
              _ => Offset.lerp(_from, _to, t),
            };
            final cardScale = switch (_phase) {
              _Phase.flying => 1 - 0.85 * t,
              _Phase.dragging when _hovered != null => 0.7,
              _ => 1.0,
            };
            return Stack(
              children: [
                // Dim the screen behind.
                Positioned.fill(
                  child: ColoredBox(
                    color: Colors.black.withValues(alpha: 0.45 * fading),
                  ),
                ),
                // What is being sent.
                if (content != null)
                  Positioned(
                    top: MediaQuery.paddingOf(context).top + 24,
                    left: 24,
                    right: 24,
                    child: Center(
                      child: Opacity(
                        opacity: fading,
                        child: _Header(
                          icon: _iconFor(content),
                          label: _error ?? content.label,
                          error: _error != null,
                        ),
                      ),
                    ),
                  ),
                // Targets.
                if (devices.isEmpty)
                  Positioned(
                    right: 24,
                    top: _size.height / 2 - 20,
                    child: Text(
                      'No device online',
                      style: text.titleMedium?.copyWith(color: Colors.white),
                    ),
                  ),
                for (final (i, device) in devices.indexed)
                  Positioned(
                    left: centres[i].dx - 48,
                    top: centres[i].dy - 48,
                    child: Opacity(
                      opacity: fading,
                      child: _Target(
                        device: device,
                        hovered: _hovered == i,
                        receiving: _phase == _Phase.flying && _hovered == i,
                        pulse: t,
                        link: palette.link,
                        ember: palette.ember,
                      ),
                    ),
                  ),
                // The card under the finger.
                if (content != null && cardAt != null)
                  Positioned(
                    left: cardAt.dx - 80,
                    top: cardAt.dy - 56,
                    child: Opacity(
                      opacity: _phase == _Phase.flying ? 1 - t * 0.6 : fading,
                      child: Transform.rotate(
                        angle: _phase == _Phase.dragging ? -0.05 : 0,
                        child: Transform.scale(
                          scale: cardScale,
                          child: _Card(
                            content: content,
                            surface: scheme.surface,
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            );
          },
        );
      },
    );
  }

  IconData _iconFor(FlingContent content) => switch (content) {
    FlingFile() => Icons.description_outlined,
    FlingPage() => Icons.public,
    FlingScreenshot() => Icons.screenshot_monitor_outlined,
  };
}

class _Header extends StatelessWidget {
  const _Header({required this.icon, required this.label, required this.error});

  final IconData icon;
  final String label;
  final bool error;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 10),
      decoration: BoxDecoration(
        color: error ? scheme.errorContainer : scheme.surface,
        borderRadius: BorderRadius.circular(24),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 20),
          const SizedBox(width: 10),
          Flexible(
            child: Text(
              label,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
        ],
      ),
    );
  }
}

class _Target extends StatelessWidget {
  const _Target({
    required this.device,
    required this.hovered,
    required this.receiving,
    required this.pulse,
    required this.link,
    required this.ember,
  });

  final EcDevice device;
  final bool hovered;
  final bool receiving;
  final double pulse;
  final Color link;
  final Color ember;

  @override
  Widget build(BuildContext context) {
    final ring = receiving ? ember : link;
    final grow = receiving
        ? 1 + 0.12 * math.sin(pulse * math.pi)
        : (hovered ? 1.12 : 1.0);
    return SizedBox(
      width: 96,
      child: Column(
        children: [
          AnimatedScale(
            scale: grow,
            duration: const Duration(milliseconds: 120),
            child: Container(
              width: 72,
              height: 72,
              decoration: BoxDecoration(
                color: Colors.white,
                shape: BoxShape.circle,
                border: Border.all(
                  color: ring,
                  width: hovered || receiving ? 4 : 2,
                ),
              ),
              child: Icon(
                _deviceIcon(device.name),
                size: 32,
                color: EcColors.ink,
              ),
            ),
          ),
          const SizedBox(height: 6),
          Text(
            device.name,
            maxLines: 2,
            textAlign: TextAlign.center,
            overflow: TextOverflow.ellipsis,
            style: const TextStyle(color: Colors.white, fontSize: 12),
          ),
        ],
      ),
    );
  }

  static IconData _deviceIcon(String name) {
    final n = name.toLowerCase();
    if (n.contains('mac') ||
        n.contains('book') ||
        n.contains('windows') ||
        n.contains('pc')) {
      return Icons.laptop_mac;
    }
    if (n.contains('tab') || n.contains('pad')) return Icons.tablet_android;
    return Icons.smartphone;
  }
}

class _Card extends StatelessWidget {
  const _Card({required this.content, required this.surface});

  final FlingContent content;
  final Color surface;

  @override
  Widget build(BuildContext context) {
    final Widget inner = switch (content) {
      FlingScreenshot(:final path) => Image.file(
        File(path),
        fit: BoxFit.cover,
        errorBuilder: (_, _, _) =>
            const Icon(Icons.screenshot_monitor_outlined, size: 40),
      ),
      FlingPage(:final title, :final url) => _Labelled(
        Icons.public,
        title.isEmpty ? url : title,
      ),
      FlingFile(:final name) => _Labelled(Icons.description_outlined, name),
    };
    return Container(
      width: 160,
      height: 112,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        color: surface,
        borderRadius: BorderRadius.circular(14),
        boxShadow: const [BoxShadow(blurRadius: 24, color: Colors.black45)],
      ),
      child: inner,
    );
  }
}

class _Labelled extends StatelessWidget {
  const _Labelled(this.icon, this.label);

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(icon, size: 32),
          const SizedBox(height: 8),
          Text(
            label,
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}
