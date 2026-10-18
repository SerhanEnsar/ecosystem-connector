import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' show ImageFilter, lerpDouble;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../controller.dart';
import '../model.dart';
import '../theme.dart';

/// A finger that started outside the overlay, fed by the platform. On Android the
/// edge handle keeps the touch that began on it and forwards the positions here, so
/// one continuous press-hold-drag works. The overlay also takes touches of its own:
/// the card can be dragged directly once it rests.
class FlingGesture extends ChangeNotifier {
  FlingContent? _content;
  Offset? _position;
  Offset? _released;

  /// What is being flung, once the hold started.
  FlingContent? get content => _content;

  /// Where the finger is, in logical pixels; null if no finger is being forwarded.
  Offset? get position => _position;

  /// Where the finger was lifted, once it was.
  Offset? get released => _released;

  /// The hold started with [content], with the finger at [at] if it is still down.
  void start(FlingContent content, [Offset? at]) {
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

/// The full-screen fling layer.
///
/// The screenshot (or page) starts at full size and shrinks into a card in the middle
/// of the screen, while a soft rounded drop zone rises at the bottom edge, split into
/// one section per online device. Dragging the card into a section sends it there: it
/// flows into the section, which shows a tick. Letting go anywhere else springs the
/// card back so it can be dragged again; a tap outside the card closes the layer.
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

  /// Share of the screen the resting card takes.
  static const restScale = 0.55;

  /// Card scale (relative to resting) while it is picked up.
  static const liftedScale = 0.6;

  /// Card scale while it is over the drop zone.
  static const hoverScale = 0.25;

  /// How much further out than the drawn zone a drop still counts.
  static const zoneSlack = 28.0;

  /// Where the card rests: centred, a little above the middle.
  static Rect restRect(Size size) => Rect.fromCenter(
    center: Offset(size.width / 2, size.height * 0.42),
    width: size.width * restScale,
    height: size.height * restScale,
  );

  /// The drop zone along the bottom edge, above the system navigation area.
  static Rect dropZone(Size size, EdgeInsets padding) {
    final width = math.min(size.width - 48, 760.0);
    const height = 128.0;
    final bottom = size.height - padding.bottom - 28;
    return Rect.fromLTWH(
      (size.width - width) / 2,
      bottom - height,
      width,
      height,
    );
  }

  /// The zone split into [count] equal sections, left to right.
  static List<Rect> sections(Rect zone, int count) {
    final width = zone.width / math.max(count, 1);
    return [
      for (var i = 0; i < count; i++)
        Rect.fromLTWH(zone.left + i * width, zone.top, width, zone.height),
    ];
  }

  @override
  State<FlingOverlay> createState() => _FlingOverlayState();
}

/// Where the card is and how it looks.
@immutable
class _Pose {
  const _Pose(this.centre, this.scale, {this.radius = 24, this.opacity = 1});

  final Offset centre;

  /// Relative to the resting size.
  final double scale;
  final double radius;
  final double opacity;

  static _Pose lerp(_Pose a, _Pose b, double t) => _Pose(
    Offset.lerp(a.centre, b.centre, t)!,
    lerpDouble(a.scale, b.scale, t)!,
    radius: lerpDouble(a.radius, b.radius, t)!,
    opacity: lerpDouble(a.opacity, b.opacity, t)!.clamp(0.0, 1.0),
  );
}

enum _Phase {
  waiting,
  resting,
  dragging,
  settling,
  sending,
  sent,
  failed,
  leaving,
}

class _FlingOverlayState extends State<FlingOverlay>
    with TickerProviderStateMixin {
  /// Moves the card between poses (shrinking in, springing back, flowing out).
  late final AnimationController _move = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 400),
  );

  /// Fades the dimming, the header and the drop zone in and out.
  late final AnimationController _chrome = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 380),
  );

  /// Picks the card up when a drag starts: it glides under the finger.
  late final AnimationController _pickup = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 180),
  );

  /// Shrinks the card while it is over the drop zone.
  late final AnimationController _hover = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 200),
  );

  var _phase = _Phase.waiting;
  var _pose = const _Pose(Offset.zero, 1);
  var _from = const _Pose(Offset.zero, 1);
  var _to = const _Pose(Offset.zero, 1);
  Curve _curve = Curves.linear;

  _Pose? _dragStart;
  Offset? _pointer;
  var _forwarded = false;
  int? _hovered;
  int? _sentTo;
  String? _error;
  var _finished = false;

  Size _size = Size.zero;
  EdgeInsets _padding = EdgeInsets.zero;
  var _reduceMotion = false;

  List<EcDevice> get _online =>
      widget.controller.devices.where((d) => d.online).toList();

  Rect get _zone => FlingOverlay.dropZone(_size, _padding);

  Rect get _rest => FlingOverlay.restRect(_size);

  Duration _ms(int ms) => Duration(milliseconds: _reduceMotion ? 1 : ms);

  @override
  void initState() {
    super.initState();
    widget.gesture.addListener(_onGesture);
    if (widget.gesture.content != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _onGesture());
    }
  }

  @override
  void dispose() {
    widget.gesture.removeListener(_onGesture);
    _move.dispose();
    _chrome.dispose();
    _pickup.dispose();
    _hover.dispose();
    super.dispose();
  }

  /// The pose drawn right now.
  _Pose _current() {
    if (_phase == _Phase.dragging && _dragStart != null && _pointer != null) {
      final p = Curves.easeOutCubic.transform(_pickup.value);
      // The card shrinks as it nears the zone, and is smallest over it.
      final near = (1 - (_zone.top - _pointer!.dy) / (_size.height * 0.35))
          .clamp(0.0, 1.0);
      final h = math.max(
        Curves.easeOut.transform(_hover.value),
        Curves.easeIn.transform(near),
      );
      final scale = lerpDouble(
        _dragStart!.scale,
        lerpDouble(FlingOverlay.liftedScale, FlingOverlay.hoverScale, h)!,
        p,
      )!;
      // Over the zone the card floats just above the finger, so the section under
      // it stays readable.
      final lift = h * (_rest.height * scale / 2 + 16);
      return _Pose(
        Offset.lerp(_dragStart!.centre, _pointer! - Offset(0, lift), p)!,
        scale,
      );
    }
    if (_move.isAnimating) {
      return _Pose.lerp(_from, _to, _curve.transform(_move.value));
    }
    return _pose;
  }

  Future<void> _animateTo(_Pose to, int ms, Curve curve) async {
    _from = _current();
    _to = to;
    _curve = curve;
    _move.duration = _ms(ms);
    await _move.forward(from: 0);
    _pose = to;
  }

  _Pose get _restPose => _Pose(_rest.center, 1);

  Rect _cardRect(_Pose pose) => Rect.fromCenter(
    center: pose.centre,
    width: _rest.width * pose.scale,
    height: _rest.height * pose.scale,
  );

  int? _sectionAt(Offset point) {
    final count = _online.length;
    final zone = _zone;
    if (count == 0 || !zone.inflate(FlingOverlay.zoneSlack).contains(point)) {
      return null;
    }
    final index = ((point.dx - zone.left) / (zone.width / count)).floor();
    return index.clamp(0, count - 1);
  }

  // The layer opens: the screenshot shrinks from full screen into the card.
  Future<void> _open() async {
    setState(() => _phase = _Phase.resting);
    _pose = _Pose(
      _size.center(Offset.zero),
      1 / FlingOverlay.restScale,
      radius: 0,
    );
    _chrome.duration = _ms(380);
    unawaited(_chrome.forward());
    await _animateTo(_restPose, 480, Curves.easeOutCubic);
  }

  void _onGesture() {
    if (widget.gesture.content == null || _size.isEmpty) return;
    if (_phase == _Phase.waiting) {
      _open();
      _forwardStart = widget.gesture.position;
    }
    final at = widget.gesture.position;
    if (at == null) return;
    if (widget.gesture.released != null) {
      if (_phase == _Phase.dragging && _forwarded) _drop(at);
      return;
    }
    if (_phase == _Phase.dragging && _forwarded) {
      _dragTo(at);
    } else if (_phase == _Phase.resting &&
        _forwardStart != null &&
        (at - _forwardStart!).distance > 8) {
      _beginDrag(at, forwarded: true);
    }
  }

  Offset? _forwardStart;

  void _beginDrag(Offset at, {required bool forwarded}) {
    _dragStart = _current();
    _move.stop();
    _pointer = at;
    _forwarded = forwarded;
    _pickup.duration = _ms(180);
    _pickup.forward(from: 0);
    HapticFeedback.selectionClick();
    setState(() => _phase = _Phase.dragging);
  }

  void _dragTo(Offset at) {
    final section = _sectionAt(at);
    if (section != _hovered) {
      _hover.duration = _ms(200);
      section == null ? _hover.reverse() : _hover.forward();
      if (section != null) HapticFeedback.selectionClick();
    }
    setState(() {
      _pointer = at;
      _hovered = section;
    });
  }

  Future<void> _drop(Offset at) async {
    final section = _sectionAt(at);
    if (section != null) return _send(section);
    // Not on a device: spring back and wait.
    _pose = _current();
    setState(() {
      _phase = _Phase.settling;
      _hovered = null;
    });
    _hover.reverse();
    await _animateTo(_restPose, 460, Curves.easeOutBack);
    if (!mounted || _phase != _Phase.settling) return;
    setState(() {
      _phase = _Phase.resting;
      _forwardStart = null;
    });
  }

  Future<void> _send(int index) async {
    final device = _online[index];
    final target = FlingOverlay.sections(_zone, _online.length)[index];
    _pose = _current();
    setState(() {
      _phase = _Phase.sending;
      _hovered = index;
    });
    HapticFeedback.mediumImpact();
    final first = widget.controller
        .fling(device, widget.gesture.content!)
        .firstWhere((s) => s is! SendProgress || s.sent > 0 || s.total == 0)
        .timeout(
          const Duration(seconds: 8),
          onTimeout: () => const SendRetrying('slow'),
        );
    await _animateTo(
      _Pose(target.center, 0.04, radius: 12, opacity: 0),
      380,
      Curves.easeInCubic,
    );
    if (!mounted) return;
    setState(() {
      _phase = _Phase.sent;
      _sentTo = index;
    });
    final status = await first.catchError((Object e) => SendFailed('$e'));
    if (!mounted) return;
    if (status is SendFailed) {
      setState(() {
        _phase = _Phase.failed;
        _sentTo = null;
        _error = 'Could not send to ${device.name}: ${status.reason}';
      });
      await Future<void>.delayed(const Duration(seconds: 2));
      return _leave(FlingResult.failed);
    }
    // Large files keep going in the background; the layer can close.
    await Future<void>.delayed(_ms(550));
    return _leave(FlingResult.sent);
  }

  void _tapOutside(Offset at) {
    if (_phase != _Phase.resting) return;
    if (_cardRect(_current()).contains(at)) return;
    if (_zone.contains(at) && _online.isNotEmpty) return;
    _leave(FlingResult.cancelled);
  }

  Future<void> _leave(FlingResult result) async {
    if (_finished || !mounted) return;
    _finished = true;
    final pose = _current();
    _pose = pose;
    setState(() => _phase = _Phase.leaving);
    _chrome.duration = _ms(260);
    await Future.wait([
      _chrome.reverse(),
      if (result == FlingResult.cancelled)
        _animateTo(
          _Pose(pose.centre, pose.scale * 0.92, opacity: 0),
          240,
          Curves.easeIn,
        ),
    ]);
    if (mounted) widget.onFinished(result);
  }

  @override
  Widget build(BuildContext context) {
    final palette = EcPalette.of(context);
    final content = widget.gesture.content;
    _reduceMotion = MediaQuery.of(context).disableAnimations;
    _padding = MediaQuery.paddingOf(context);

    return LayoutBuilder(
      builder: (context, constraints) {
        final firstLayout = _size.isEmpty;
        _size = constraints.biggest;
        if (firstLayout && content != null) {
          WidgetsBinding.instance.addPostFrameCallback((_) => _onGesture());
        }
        final devices = _online;
        final zone = _zone;
        final sections = FlingOverlay.sections(zone, devices.length);
        return GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapUp: (d) => _tapOutside(d.localPosition),
          onPanStart: (d) {
            if (_phase == _Phase.resting &&
                _cardRect(_current()).contains(d.localPosition)) {
              _beginDrag(d.localPosition, forwarded: false);
            }
          },
          onPanUpdate: (d) {
            if (_phase == _Phase.dragging && !_forwarded) {
              _dragTo(d.localPosition);
            }
          },
          onPanEnd: (_) {
            if (_phase == _Phase.dragging && !_forwarded && _pointer != null) {
              _drop(_pointer!);
            }
          },
          child: AnimatedBuilder(
            animation: Listenable.merge([_move, _chrome, _pickup, _hover]),
            builder: (context, _) {
              if (content == null || _phase == _Phase.waiting) {
                return const SizedBox.expand();
              }
              final chrome = Curves.easeOut.transform(_chrome.value);
              final pose = _current();
              final card = _cardRect(pose);
              return Stack(
                children: [
                  Positioned.fill(
                    child: ColoredBox(
                      color: Colors.black.withValues(alpha: 0.6 * chrome),
                    ),
                  ),
                  // What is being sent, or what went wrong.
                  Positioned(
                    top: _padding.top + 20,
                    left: 24,
                    right: 24,
                    child: Center(
                      child: Opacity(
                        opacity: chrome,
                        child: _Header(
                          icon: _iconFor(content),
                          label: _error ?? content.label,
                          error: _error != null,
                        ),
                      ),
                    ),
                  ),
                  // The drop zone rises from the bottom edge.
                  Positioned.fromRect(
                    rect: zone.translate(0, 36 * (1 - chrome)),
                    child: Opacity(
                      opacity: chrome,
                      child: _DropZone(
                        devices: devices,
                        sections: [
                          for (final s in sections) s.shift(-zone.topLeft),
                        ],
                        hovered:
                            _phase == _Phase.dragging ||
                                _phase == _Phase.sending
                            ? _hovered
                            : null,
                        glow: _hover.value,
                        sentTo: _sentTo,
                        link: palette.link,
                      ),
                    ),
                  ),
                  Positioned(
                    left: 24,
                    right: 24,
                    top: zone.top - 34,
                    child: Opacity(
                      opacity: chrome * (_phase == _Phase.resting ? 1 : 0.0),
                      child: Text(
                        devices.isEmpty
                            ? 'Tap anywhere to close'
                            : 'Drag onto a device · tap outside to close',
                        textAlign: TextAlign.center,
                        style: TextStyle(
                          color: Colors.white.withValues(alpha: 0.8),
                          fontSize: 13,
                        ),
                      ),
                    ),
                  ),
                  // The card.
                  Positioned.fromRect(
                    rect: card,
                    child: IgnorePointer(
                      child: Opacity(
                        opacity: pose.opacity,
                        child: _Card(content: content, radius: pose.radius),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        );
      },
    );
  }

  IconData _iconFor(FlingContent content) => switch (content) {
    FlingFile() => Icons.description_outlined,
    FlingFiles(:final paths) =>
      paths.length == 1 ? Icons.description_outlined : Icons.file_copy_outlined,
    FlingText() => Icons.notes,
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

/// The frosted, rounded area at the bottom, one section per online device.
class _DropZone extends StatelessWidget {
  const _DropZone({
    required this.devices,
    required this.sections,
    required this.hovered,
    required this.glow,
    required this.sentTo,
    required this.link,
  });

  final List<EcDevice> devices;

  /// Section rectangles, relative to the zone.
  final List<Rect> sections;
  final int? hovered;
  final double glow;
  final int? sentTo;
  final Color link;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(28);
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: shape,
        boxShadow: [
          BoxShadow(
            color: link.withValues(alpha: 0.10 + 0.25 * glow),
            blurRadius: 28 + 20 * glow,
            spreadRadius: 2 * glow,
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: shape,
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
          child: DecoratedBox(
            decoration: BoxDecoration(
              // Other apps behind the layer cannot be blurred, so the tint does
              // the work of keeping the labels readable.
              color: const Color(0xFF1B2433).withValues(alpha: 0.86),
              borderRadius: shape,
              border: Border.all(
                color: Color.lerp(
                  Colors.white.withValues(alpha: 0.2),
                  link,
                  glow,
                )!,
                width: 1 + glow,
              ),
            ),
            child: devices.isEmpty
                ? Center(
                    child: Text(
                      'No device online',
                      style: TextStyle(
                        color: Colors.white.withValues(alpha: 0.75),
                        fontSize: 15,
                      ),
                    ),
                  )
                : Stack(
                    children: [
                      for (final (i, device) in devices.indexed)
                        Positioned.fromRect(
                          rect: sections[i],
                          child: _Section(
                            device: device,
                            hovered: hovered == i,
                            sent: sentTo == i,
                            divider: i > 0,
                            link: link,
                          ),
                        ),
                    ],
                  ),
          ),
        ),
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section({
    required this.device,
    required this.hovered,
    required this.sent,
    required this.divider,
    required this.link,
  });

  final EcDevice device;
  final bool hovered;
  final bool sent;
  final bool divider;
  final Color link;

  @override
  Widget build(BuildContext context) {
    return AnimatedContainer(
      duration: const Duration(milliseconds: 180),
      margin: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: hovered || sent
            ? link.withValues(alpha: 0.26)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(20),
        border: divider && !hovered && !sent
            ? Border(
                left: BorderSide(color: Colors.white.withValues(alpha: 0.12)),
              )
            : null,
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          AnimatedSwitcher(
            duration: const Duration(milliseconds: 220),
            transitionBuilder: (child, animation) =>
                ScaleTransition(scale: animation, child: child),
            child: Icon(
              sent ? Icons.check_circle : _deviceIcon(device.name),
              key: ValueKey(sent),
              size: 34,
              color: sent ? link : Colors.white,
            ),
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Text(
              sent ? 'Sent' : device.name,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                color: Colors.white,
                fontSize: 14,
                fontWeight: hovered ? FontWeight.w600 : FontWeight.w500,
              ),
            ),
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

/// The screenshot or page being flung, with rounded corners and a soft shadow.
class _Card extends StatelessWidget {
  const _Card({required this.content, required this.radius});

  final FlingContent content;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final surface = Theme.of(context).colorScheme.surface;
    final preview = switch (content) {
      FlingScreenshot(:final path) => path,
      FlingPage(:final preview) => preview,
      FlingFiles(:final paths)
          when paths.length == 1 && _isImage(paths.single) =>
        paths.single,
      FlingFile() || FlingFiles() || FlingText() => null,
    };
    final Widget inner = preview != null
        ? Stack(
            fit: StackFit.expand,
            children: [
              Image.file(
                File(preview),
                fit: BoxFit.cover,
                errorBuilder: (_, _, _) => ColoredBox(color: surface),
              ),
              if (content case FlingPage(:final title, :final url))
                Align(
                  alignment: Alignment.bottomCenter,
                  child: _Caption(title.isEmpty ? url : title),
                ),
            ],
          )
        : ColoredBox(
            color: surface,
            child: switch (content) {
              FlingPage(:final title, :final url) => _Labelled(
                Icons.public,
                title.isEmpty ? url : title,
              ),
              FlingFile(:final name) => _Labelled(
                Icons.description_outlined,
                name,
              ),
              FlingFiles(:final name, :final paths) => _Labelled(
                paths.length == 1
                    ? Icons.description_outlined
                    : Icons.file_copy_outlined,
                name,
              ),
              FlingText(:final text) => _TextPreview(text),
              FlingScreenshot() => const Icon(
                Icons.screenshot_monitor_outlined,
                size: 40,
              ),
            },
          );
    final shape = BorderRadius.circular(radius);
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: shape,
        boxShadow: const [
          BoxShadow(
            blurRadius: 32,
            color: Colors.black54,
            offset: Offset(0, 12),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: shape,
        child: DecoratedBox(
          position: DecorationPosition.foreground,
          decoration: BoxDecoration(
            borderRadius: shape,
            border: Border.all(color: Colors.white.withValues(alpha: 0.18)),
          ),
          child: inner,
        ),
      ),
    );
  }
}

bool _isImage(String path) => RegExp(
  r'\.(jpe?g|png|webp|gif|bmp|heic)$',
  caseSensitive: false,
).hasMatch(path);

/// The start of the text, as on a note card.
class _TextPreview extends StatelessWidget {
  const _TextPreview(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(28),
      child: Align(
        alignment: Alignment.topLeft,
        child: Text(
          text,
          overflow: TextOverflow.fade,
          style: Theme.of(context).textTheme.titleLarge,
        ),
      ),
    );
  }
}

class _Caption extends StatelessWidget {
  const _Caption(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(16, 28, 16, 14),
      decoration: const BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
          colors: [Colors.transparent, Colors.black87],
        ),
      ),
      child: Row(
        children: [
          const Icon(Icons.public, color: Colors.white, size: 18),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              text,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: const TextStyle(color: Colors.white, fontSize: 14),
            ),
          ),
        ],
      ),
    );
  }
}

class _Labelled extends StatelessWidget {
  const _Labelled(this.icon, this.label);

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return FittedBox(
      fit: BoxFit.scaleDown,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(icon, size: 40),
            const SizedBox(height: 10),
            ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 280),
              child: Text(
                label,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                textAlign: TextAlign.center,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
