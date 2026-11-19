import 'dart:async';

import 'package:flutter/material.dart';

import 'arrival_card.dart';

/// One card for [ArrivalLayer]: what it shows and what its buttons do.
class ArrivalEntry {
  const ArrivalEntry({
    required this.data,
    required this.onPrimary,
    this.onOpen,
    this.onSaveElsewhere,
  });

  final ArrivalCardData data;

  /// "Show in folder", "Open in browser" or "Copy", as the card's data says.
  final VoidCallback onPrimary;

  /// "Open" for a file.
  final VoidCallback? onOpen;

  /// "Save to…" for a file.
  final VoidCallback? onSaveElsewhere;
}

/// Draws arrival cards at the top of [child], one under the other, the way the macOS
/// app stacks its cards. A card removes itself when it has left.
///
/// The cards only cover this window: they cannot show over other apps.
class ArrivalLayer extends StatefulWidget {
  const ArrivalLayer({super.key, required this.arrivals, required this.child});

  /// Arrivals to show, in the order they come.
  final Stream<ArrivalEntry> arrivals;

  final Widget child;

  /// Distance between the tops of two stacked cards.
  static const step = ArrivalCard.height - 24;

  /// Distance from the top of the window to the first card.
  static const top = 30.0;

  @override
  State<ArrivalLayer> createState() => _ArrivalLayerState();
}

class _ArrivalLayerState extends State<ArrivalLayer> {
  final _cards = <(int, ArrivalEntry)>[];
  var _nextId = 0;
  late StreamSubscription<ArrivalEntry> _subscription;

  @override
  void initState() {
    super.initState();
    _subscription = widget.arrivals.listen((entry) {
      setState(() => _cards.add((_nextId++, entry)));
    });
  }

  @override
  void didUpdateWidget(ArrivalLayer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.arrivals != widget.arrivals) {
      _subscription.cancel();
      _subscription = widget.arrivals.listen((entry) {
        setState(() => _cards.add((_nextId++, entry)));
      });
    }
  }

  @override
  void dispose() {
    _subscription.cancel();
    super.dispose();
  }

  void _remove(int id) {
    setState(() => _cards.removeWhere((card) => card.$1 == id));
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        widget.child,
        for (final (slot, (id, entry)) in _cards.indexed)
          Positioned(
            key: ValueKey(id),
            top: ArrivalLayer.top + slot * ArrivalLayer.step,
            left: 0,
            right: 0,
            child: Center(
              child: SizedBox(
                width: ArrivalCard.width,
                height: ArrivalCard.height,
                // Above the app's navigator there is no Material, which the card's
                // buttons need for their ink.
                child: Material(
                  type: MaterialType.transparency,
                  child: ArrivalCard(
                    data: entry.data,
                    onPrimary: entry.onPrimary,
                    onOpen: entry.onOpen,
                    onSaveElsewhere: entry.onSaveElsewhere,
                    onClosed: () => _remove(id),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}
