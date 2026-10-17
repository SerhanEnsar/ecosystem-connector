import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Talks to FlingHandle.kt, which forwards the finger's position.
const _channel = MethodChannel(
  'io.github.serhanensar.ecosystemconnector/fling',
);

/// The transparent, full-screen fling layer. Each hold of the edge handle starts a
/// fresh [FlingOverlay]; between holds it draws nothing.
class FlingLayer extends StatefulWidget {
  const FlingLayer({super.key, required this.controller});

  final EcController controller;

  @override
  State<FlingLayer> createState() => _FlingLayerState();
}

class _FlingLayerState extends State<FlingLayer> {
  FlingGesture? _gesture;
  var _round = 0;

  @override
  void initState() {
    super.initState();
    _channel.setMethodCallHandler(_onCall);
  }

  /// Native positions are in physical pixels.
  Offset _at(Map<Object?, Object?> args) {
    final ratio =
        WidgetsBinding.instance.platformDispatcher.views.first.devicePixelRatio;
    return Offset(
      (args['x'] as num).toDouble() / ratio,
      (args['y'] as num).toDouble() / ratio,
    );
  }

  Future<void> _onCall(MethodCall call) async {
    final args = (call.arguments as Map?) ?? const {};
    switch (call.method) {
      case 'start':
        final content = switch (args['type']) {
          'page' => FlingPage(
            args['url'] as String,
            title: (args['title'] as String?) ?? '',
          ),
          'file' => FlingFile(
            args['path'] as String,
            page: args['page'] as int?,
          ),
          _ => FlingScreenshot(args['path'] as String),
        };
        setState(() {
          _round++;
          _gesture = FlingGesture()..start(content, _at(args));
        });
      case 'move':
        _gesture?.move(_at(args));
      case 'release':
        _gesture?.release(_at(args));
    }
  }

  void _finished(FlingResult _) {
    setState(() => _gesture = null);
    _channel.invokeMethod<void>('finished');
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
        child: gesture == null
            ? const SizedBox.expand()
            : FlingOverlay(
                key: ValueKey(_round),
                controller: widget.controller,
                gesture: gesture,
                onFinished: _finished,
              ),
      ),
    );
  }
}
