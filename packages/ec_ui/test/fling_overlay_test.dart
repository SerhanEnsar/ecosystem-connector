import 'package:ec_ui/ec_ui.dart';
import 'package:ec_ui/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

final mac = EcDevice(
  id: 'm' * 64,
  shortId: '2BD4-3BD1',
  name: "Serhan's Mac",
  online: true,
);
final phone = EcDevice(
  id: 'p' * 64,
  shortId: '1111-2222',
  name: 'Pixel',
  online: false,
);

const size = Size(800, 1200);

Future<(FakeController, FlingGesture, List<FlingResult>)> pumpOverlay(
  WidgetTester tester, {
  List<EcDevice>? devices,
}) async {
  tester.view.physicalSize = size;
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.reset);
  final controller = FakeController(devices: devices ?? [mac, phone]);
  final gesture = FlingGesture();
  final results = <FlingResult>[];
  await tester.pumpWidget(
    MaterialApp(
      theme: ecLightTheme(),
      home: Scaffold(
        backgroundColor: Colors.transparent,
        body: FlingOverlay(
          controller: controller,
          gesture: gesture,
          onFinished: results.add,
        ),
      ),
    ),
  );
  return (controller, gesture, results);
}

Offset target(int index, int count) =>
    FlingOverlay.targetCentres(size, count)[index];

void main() {
  testWidgets('the header names what is being sent', (tester) async {
    final (_, gesture, _) = await pumpOverlay(tester);
    gesture.start(
      const FlingPage('https://example.com', title: 'Lecture 3'),
      const Offset(20, 600),
    );
    await tester.pump();
    expect(find.text('Web page · Lecture 3'), findsOneWidget);
  });

  testWidgets('only online devices are targets', (tester) async {
    final (_, gesture, _) = await pumpOverlay(tester);
    gesture.start(const FlingScreenshot('/nope.png'), const Offset(20, 600));
    await tester.pump();
    expect(find.text("Serhan's Mac"), findsOneWidget);
    expect(find.text('Pixel'), findsNothing);
  });

  testWidgets('dropping on a device flings the content to it', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(tester);
    const content = FlingFile('/docs/slides.pdf', page: 12);
    gesture.start(content, const Offset(20, 600));
    gesture.move(const Offset(400, 600));
    gesture.release(target(0, 1));
    await tester.pump();
    expect(controller.flings.single.$1.id, mac.id);
    expect(controller.flings.single.$2, content);

    controller.sending.add(
      const SendProgress(sent: 1000, total: 2000, paths: 1),
    );
    await tester.pumpAndSettle();
    // The overlay lingers briefly after the card lands.
    await tester.pump(const Duration(milliseconds: 300));
    expect(results, [FlingResult.sent]);
  });

  testWidgets('letting go anywhere else cancels', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(tester);
    gesture.start(const FlingScreenshot('/nope.png'), const Offset(20, 600));
    gesture.move(const Offset(300, 300));
    gesture.release(const Offset(300, 300));
    await tester.pumpAndSettle();
    expect(controller.flings, isEmpty);
    expect(results, [FlingResult.cancelled]);
  });

  testWidgets('a failed fling explains why', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(tester);
    gesture.start(const FlingScreenshot('/nope.png'), const Offset(20, 600));
    gesture.release(target(0, 1));
    await tester.pump();
    controller.sending.add(
      const SendFailed('the device is not on the network'),
    );
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.textContaining('Could not send to'), findsOneWidget);
    await tester.pumpAndSettle(const Duration(seconds: 3));
    expect(results, [FlingResult.failed]);
  });

  testWidgets('with no device online, letting go cancels', (tester) async {
    final (_, gesture, results) = await pumpOverlay(tester, devices: [phone]);
    gesture.start(const FlingScreenshot('/nope.png'), const Offset(20, 600));
    await tester.pump();
    expect(find.text('No device online'), findsOneWidget);
    gesture.release(const Offset(700, 600));
    await tester.pumpAndSettle();
    expect(results, [FlingResult.cancelled]);
  });
}
