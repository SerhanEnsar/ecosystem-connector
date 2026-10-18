import 'dart:async';

import 'package:ec_app/ec_app.dart';
import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Talks to FlingHandle.kt, which forwards the finger's position and sizes the window.
const _channel = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/fling',
);

/// Arrivals to show (files received, pages flung here). Fed by this engine's
/// controller through `PlatformHooks.showArrival`.
final arrivals = StreamController<ArrivalInfo>.broadcast();

/// The transparent layer drawn over other apps. It shows the fling overlay while the
/// edge handle is held, and arrival cards at the top when something comes in. It
/// tells FlingHandle.kt which window it needs: the whole screen, a strip at the top,
/// or none.
class FlingLayer extends StatefulWidget {
  const FlingLayer({super.key, required this.controller});

  final EcController controller;

  @override
  State<FlingLayer> createState() => _FlingLayerState();
}

class _FlingLayerState extends State<FlingLayer> {
  FlingGesture? _gesture;
  var _round = 0;
  final _cards = <(int, ArrivalInfo)>[];
  var _nextCard = 0;
  var _mode = 'none';
  late final StreamSubscription<ArrivalInfo> _arrivals;

  @override
  void initState() {
    super.initState();
    _channel.setMethodCallHandler(_onCall);
    _arrivals = arrivals.stream.listen((arrival) {
      setState(() => _cards.add((_nextCard++, arrival)));
      _syncWindow();
    });
  }

  @override
  void dispose() {
    _arrivals.cancel();
    super.dispose();
  }

  void _syncWindow() {
    final mode = _gesture != null
        ? 'fling'
        : _cards.isNotEmpty
        ? 'arrival'
        : 'none';
    if (mode == _mode) return;
    _mode = mode;
    _channel.invokeMethod<void>('layer', mode);
  }

  /// Native positions are in physical pixels; null when no finger is down.
  Offset? _at(Map<Object?, Object?> args) {
    final (x, y) = (args['x'] as num?, args['y'] as num?);
    if (x == null || y == null) return null;
    final ratio =
        WidgetsBinding.instance.platformDispatcher.views.first.devicePixelRatio;
    return Offset(x.toDouble() / ratio, y.toDouble() / ratio);
  }

  Future<void> _onCall(MethodCall call) async {
    final args = (call.arguments as Map?) ?? const {};
    switch (call.method) {
      case 'start':
        final content = switch (args['type']) {
          'page' => FlingPage(
            args['url'] as String,
            title: (args['title'] as String?) ?? '',
            preview: args['path'] as String?,
          ),
          'file' => FlingFile(
            args['path'] as String,
            page: args['page'] as int?,
          ),
          _ => FlingScreenshot(args['path'] as String),
        };
        // FlingHandle.kt already made the window full-screen for the fling.
        _mode = 'fling';
        setState(() {
          _round++;
          _gesture = FlingGesture()..start(content, _at(args));
        });
      case 'move':
        if (_at(args) case final at?) _gesture?.move(at);
      case 'release':
        if (_at(args) case final at?) _gesture?.release(at);
    }
  }

  void _flingFinished(FlingResult _) {
    setState(() => _gesture = null);
    _syncWindow();
  }

  void _cardClosed(int id) {
    setState(() => _cards.removeWhere((c) => c.$1 == id));
    _syncWindow();
  }

  ArrivalCardData _cardData(ArrivalInfo a) {
    final single = a.paths.length == 1 ? a.paths.single : null;
    final isImage =
        single != null &&
        RegExp(
          r'\.(jpe?g|png|webp|gif|bmp)$',
          caseSensitive: false,
        ).hasMatch(single);
    return ArrivalCardData(
      title: a.displayTitle,
      from: a.from,
      detail: a.url != null
          ? (Uri.tryParse(a.url!)?.host ?? a.url!)
          : formatBytes(a.size),
      imagePath: isImage ? single : null,
      isPage: a.url != null,
    );
  }

  @override
  Widget build(BuildContext context) {
    final gesture = _gesture;
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      color: Colors.transparent,
      theme: ecLightTheme().copyWith(
        scaffoldBackgroundColor: Colors.transparent,
      ),
      darkTheme: ecDarkTheme().copyWith(
        scaffoldBackgroundColor: Colors.transparent,
      ),
      home: Material(
        type: MaterialType.transparency,
        child: Stack(
          children: [
            if (gesture != null)
              Positioned.fill(
                child: FlingOverlay(
                  key: ValueKey(_round),
                  controller: widget.controller,
                  gesture: gesture,
                  onFinished: _flingFinished,
                ),
              ),
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              child: Column(
                children: [
                  for (final (id, arrival) in _cards)
                    SizedBox(
                      height: ArrivalCard.height + 36,
                      child: ArrivalCard(
                        key: ValueKey(id),
                        data: _cardData(arrival),
                        onPrimary: () => arrival.url != null
                            ? _channel.invokeMethod<bool>(
                                'openUrl',
                                arrival.url,
                              )
                            : _channel.invokeMethod<bool>('showInFolder'),
                        onOpen: arrival.paths.length == 1
                            ? () => _channel.invokeMethod<bool>(
                                'openFile',
                                arrival.paths.single,
                              )
                            : null,
                        onClosed: () => _cardClosed(id),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
