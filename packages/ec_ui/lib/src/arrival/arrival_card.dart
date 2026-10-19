import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';

/// What the arrival card shows.
class ArrivalCardData {
  const ArrivalCardData({
    required this.title,
    required this.from,
    required this.detail,
    this.imagePath,
    this.isPage = false,
    this.isText = false,
    this.canSaveElsewhere = false,
    this.linked = false,
  });

  /// File name, "3 files", the page title, or the text's first line.
  final String title;

  /// Name of the sending device.
  final String from;

  /// Size, the page's domain, or more of the text.
  final String detail;

  /// An image file to show as the preview.
  final String? imagePath;

  /// A web page rather than files.
  final bool isPage;

  /// Text to copy rather than files.
  final bool isText;

  /// Offer "Save to…".
  final bool canSaveElsewhere;

  /// A linked file: later edits on either device reach the other.
  final bool linked;
}

/// A drop that falls from the top edge and spreads into a frosted card, stays a few
/// seconds (longer while touched or hovered), then shrinks back into a drop and
/// leaves. The same design as the macOS card (ArrivalPanel.swift).
class ArrivalCard extends StatefulWidget {
  const ArrivalCard({
    super.key,
    required this.data,
    required this.onPrimary,
    required this.onClosed,
    this.onOpen,
    this.onSaveElsewhere,
    this.stay = const Duration(seconds: 3),
  });

  final ArrivalCardData data;

  /// "Show in folder" for files, "Open in browser" for a page, "Copy" for text.
  final VoidCallback onPrimary;

  /// "Open" for files, where the platform can open them.
  final VoidCallback? onOpen;

  /// "Save to…".
  final VoidCallback? onSaveElsewhere;

  /// The card has left; remove it.
  final VoidCallback onClosed;

  /// How long the card stays before leaving on its own.
  final Duration stay;

  static const width = 380.0;
  static const height = 140.0;

  @override
  State<ArrivalCard> createState() => _ArrivalCardState();
}

class _ArrivalCardState extends State<ArrivalCard>
    with TickerProviderStateMixin {
  late final AnimationController _drop = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 350),
  );
  late final AnimationController _spread = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 520),
  );
  Timer? _timer;
  var _held = false;
  var _leaving = false;

  @override
  void initState() {
    super.initState();
    _appear();
  }

  Future<void> _appear() async {
    await _drop.forward();
    if (!mounted) return;
    await _spread.forward();
    _startTimer();
  }

  void _startTimer() {
    _timer?.cancel();
    if (!_held) _timer = Timer(widget.stay, _leave);
  }

  void _hold(bool held) {
    _held = held;
    if (held) {
      _timer?.cancel();
    } else if (_spread.isCompleted) {
      _startTimer();
    }
  }

  Future<void> _leave() async {
    if (_leaving || !mounted) return;
    _leaving = true;
    _timer?.cancel();
    await _spread.reverse();
    if (!mounted) return;
    await _drop.reverse();
    widget.onClosed();
  }

  void _act(VoidCallback? action) {
    action?.call();
    _leave();
  }

  @override
  void dispose() {
    _timer?.cancel();
    _drop.dispose();
    _spread.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: Listenable.merge([_drop, _spread]),
      builder: (context, _) {
        final drop = Curves.easeOut.transform(_drop.value);
        final spread = Curves.easeOutBack
            .transform(_spread.value)
            .clamp(0.0, 1.08);
        final w = 18 + (ArrivalCard.width - 18) * spread;
        final h = 18 + (ArrivalCard.height - 18) * spread;
        final radius = 9 + 13 * spread.clamp(0.0, 1.0);
        final showContent = _spread.value > 0.85;
        return Align(
          alignment: Alignment.topCenter,
          child: Transform.translate(
            offset: Offset(0, -30 + 42 * drop),
            child: Opacity(
              opacity: drop,
              child: MouseRegion(
                onEnter: (_) => _hold(true),
                onExit: (_) => _hold(false),
                child: Listener(
                  onPointerDown: (_) => _hold(true),
                  onPointerUp: (_) => _hold(false),
                  onPointerCancel: (_) => _hold(false),
                  child: FrostedGlass(
                    width: w,
                    height: h,
                    radius: radius,
                    shadow: spread.clamp(0.0, 1.0),
                    child: showContent
                        // Laid out at full size and clipped by the card, so nothing
                        // overflows while the card is still spreading.
                        ? OverflowBox(
                            minWidth: ArrivalCard.width,
                            maxWidth: ArrivalCard.width,
                            minHeight: ArrivalCard.height,
                            maxHeight: ArrivalCard.height,
                            child: _content(context),
                          )
                        : null,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _content(BuildContext context) {
    final data = widget.data;
    const ink = Colors.white;
    return Padding(
      padding: const EdgeInsets.all(12),
      child: Row(
        children: [
          ClipRRect(
            borderRadius: BorderRadius.circular(14),
            child: SizedBox.square(dimension: 92, child: _preview()),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  data.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    color: ink,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  'From ${data.from}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: ink.withValues(alpha: 0.75),
                    fontSize: 12,
                  ),
                ),
                Row(
                  children: [
                    if (data.linked) ...[
                      Icon(
                        Icons.link,
                        size: 13,
                        color: ink.withValues(alpha: 0.75),
                      ),
                      const SizedBox(width: 4),
                    ],
                    Flexible(
                      child: Text(
                        data.linked
                            ? 'Linked, kept up to date · ${data.detail}'
                            : data.detail,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          color: ink.withValues(alpha: 0.55),
                          fontSize: 11,
                        ),
                      ),
                    ),
                  ],
                ),
                const Spacer(),
                // One row that scrolls rather than wraps, so it never overflows.
                SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      if (!data.isPage &&
                          !data.isText &&
                          widget.onOpen != null) ...[
                        FrostedPill('Open', onTap: () => _act(widget.onOpen)),
                        const SizedBox(width: 6),
                      ],
                      FrostedPill(
                        data.isPage
                            ? 'Open in browser'
                            : data.isText
                            ? 'Copy'
                            : 'Show in folder',
                        onTap: () => _act(widget.onPrimary),
                      ),
                      if (data.canSaveElsewhere &&
                          widget.onSaveElsewhere != null) ...[
                        const SizedBox(width: 6),
                        FrostedPill(
                          'Save to…',
                          quiet: true,
                          onTap: () => _act(widget.onSaveElsewhere),
                        ),
                      ],
                    ],
                  ),
                ),
              ],
            ),
          ),
          Align(
            alignment: Alignment.topCenter,
            child: Semantics(
              button: true,
              label: 'Close',
              child: InkResponse(
                onTap: _leave,
                radius: 18,
                child: Container(
                  width: 24,
                  height: 24,
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.14),
                    shape: BoxShape.circle,
                  ),
                  child: const Icon(Icons.close, size: 14, color: ink),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _preview() {
    final path = widget.data.imagePath;
    final placeholder = ColoredBox(
      color: Colors.white.withValues(alpha: 0.12),
      child: Icon(
        widget.data.isPage
            ? Icons.public
            : widget.data.isText
            ? Icons.notes
            : Icons.description_outlined,
        size: 32,
        color: Colors.white,
      ),
    );
    if (path == null) return placeholder;
    return Image.file(
      File(path),
      fit: BoxFit.cover,
      cacheWidth: 200,
      errorBuilder: (_, _, _) => placeholder,
    );
  }
}

/// Frosted glass: blurred backdrop, a dark tint for legibility, a fine light border.
/// The surface of the arrival and conflict cards.
class FrostedGlass extends StatelessWidget {
  const FrostedGlass({
    super.key,
    required this.width,
    required this.height,
    required this.radius,
    required this.shadow,
    this.child,
  });

  final double width;
  final double height;
  final double radius;
  final double shadow;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final shape = BorderRadius.circular(radius);
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        borderRadius: shape,
        boxShadow: [
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.30 * shadow),
            blurRadius: 24,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: shape,
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 24, sigmaY: 24),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: const Color(0xFF1B2433).withValues(alpha: 0.62),
              borderRadius: shape,
              border: Border.all(color: Colors.white.withValues(alpha: 0.18)),
            ),
            child: child,
          ),
        ),
      ),
    );
  }
}

/// A button on a [FrostedGlass] card.
class FrostedPill extends StatelessWidget {
  const FrostedPill(
    this.label, {
    super.key,
    required this.onTap,
    this.quiet = false,
  });

  final String label;
  final VoidCallback onTap;
  final bool quiet;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: quiet ? Colors.transparent : Colors.white.withValues(alpha: 0.16),
      shape: const StadiumBorder(),
      child: InkWell(
        customBorder: const StadiumBorder(),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
          child: Text(
            label,
            style: TextStyle(
              color: Colors.white.withValues(alpha: quiet ? 0.7 : 1),
              fontSize: 12,
              fontWeight: FontWeight.w500,
            ),
          ),
        ),
      ),
    );
  }
}
