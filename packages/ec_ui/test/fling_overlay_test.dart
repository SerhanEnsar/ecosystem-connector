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
final laptop = EcDevice(
  id: 'w' * 64,
  shortId: '3333-4444',
  name: 'Windows PC',
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
  FlingContent content = const FlingScreenshot('/nope.png'),
  Offset? finger = const Offset(10, 600),
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
  gesture.start(content, finger);
  await tester.pump();
  // The screenshot shrinks into the card.
  await tester.pumpAndSettle();
  return (controller, gesture, results);
}

Rect zone() => FlingOverlay.dropZone(size, EdgeInsets.zero);

Offset section(int index, int count) =>
    FlingOverlay.sections(zone(), count)[index].center;

Offset restCentre() => FlingOverlay.restRect(size).center;

void main() {
  test('the zone is split evenly, left to right', () {
    final parts = FlingOverlay.sections(zone(), 3);
    expect(parts, hasLength(3));
    expect(parts.first.left, zone().left);
    expect(parts.last.right, closeTo(zone().right, 0.001));
    expect(parts[1].width, closeTo(zone().width / 3, 0.001));
  });

  testWidgets('the header names what is being sent', (tester) async {
    await pumpOverlay(
      tester,
      content: const FlingPage('https://example.com', title: 'Lecture 3'),
    );
    expect(find.text('Web page · Lecture 3'), findsOneWidget);
  });

  testWidgets('each online device gets a section, offline ones none', (
    tester,
  ) async {
    await pumpOverlay(tester, devices: [mac, phone, laptop]);
    expect(find.text("Serhan's Mac"), findsOneWidget);
    expect(find.text('Windows PC'), findsOneWidget);
    expect(find.text('Pixel'), findsNothing);
  });

  testWidgets('dragging from the handle into a section sends to that device', (
    tester,
  ) async {
    final (controller, gesture, results) = await pumpOverlay(
      tester,
      devices: [mac, laptop],
    );
    const content = FlingScreenshot('/nope.png');
    gesture.move(const Offset(200, 800));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.move(section(1, 2));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(section(1, 2));
    await tester.pump();
    expect(controller.flings.single.$1.id, laptop.id);
    expect(controller.flings.single.$2, isA<FlingScreenshot>());
    expect((controller.flings.single.$2 as FlingScreenshot).path, content.path);

    controller.sending.add(
      const SendProgress(sent: 1000, total: 2000, paths: 1),
    );
    await tester.pump(const Duration(milliseconds: 400));
    expect(find.text('Sent'), findsOneWidget);
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(results, [FlingResult.sent]);
  });

  testWidgets('the card can also be dragged by touch once it rests', (
    tester,
  ) async {
    final (controller, gesture, results) = await pumpOverlay(
      tester,
      finger: null,
    );
    final start = restCentre();
    final gestureTouch = await tester.startGesture(start);
    for (var i = 1; i <= 10; i++) {
      await gestureTouch.moveTo(Offset.lerp(start, section(0, 1), i / 10)!);
      await tester.pump(const Duration(milliseconds: 16));
    }
    await gestureTouch.up();
    await tester.pump();
    expect(controller.flings.single.$1.id, mac.id);
    controller.sending.add(const SendProgress(sent: 1, total: 1, paths: 1));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
    expect(results, [FlingResult.sent]);
  });

  testWidgets('letting go elsewhere springs back and waits', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(tester);
    gesture.move(const Offset(300, 300));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(const Offset(300, 300));
    await tester.pumpAndSettle();
    expect(controller.flings, isEmpty);
    expect(results, isEmpty);
    expect(
      find.text('Drag onto a device · tap outside to close'),
      findsOneWidget,
    );
  });

  testWidgets('a tap outside the card closes the layer', (tester) async {
    final (controller, _, results) = await pumpOverlay(tester);
    await tester.tapAt(const Offset(20, 40));
    await tester.pumpAndSettle();
    expect(controller.flings, isEmpty);
    expect(results, [FlingResult.cancelled]);
  });

  testWidgets('a tap on the card does not close it', (tester) async {
    final (_, _, results) = await pumpOverlay(tester);
    await tester.tapAt(restCentre());
    await tester.pumpAndSettle();
    expect(results, isEmpty);
  });

  testWidgets('a failed fling explains why', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(tester);
    gesture.move(section(0, 1));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(section(0, 1));
    await tester.pump();
    controller.sending.add(
      const SendFailed('the device is not on the network'),
    );
    await tester.pump(const Duration(milliseconds: 500));
    expect(find.textContaining('Could not send to'), findsOneWidget);
    await tester.pumpAndSettle(const Duration(seconds: 3));
    expect(results, [FlingResult.failed]);
  });

  testWidgets('with no device online the zone says so', (tester) async {
    final (controller, gesture, results) = await pumpOverlay(
      tester,
      devices: [phone],
    );
    expect(find.text('No device online'), findsOneWidget);
    gesture.move(zone().center);
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(zone().center);
    await tester.pumpAndSettle();
    expect(controller.flings, isEmpty);
    expect(results, isEmpty);
    await tester.tapAt(zone().center);
    await tester.pumpAndSettle();
    expect(results, [FlingResult.cancelled]);
  });

  testWidgets('shared text shows on the card and is sent as text', (
    tester,
  ) async {
    final (controller, gesture, _) = await pumpOverlay(
      tester,
      content: const FlingText('Room 204, 14:00'),
    );
    expect(find.text('Text'), findsOneWidget);
    expect(find.text('Room 204, 14:00'), findsOneWidget);
    gesture.move(section(0, 1));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(section(0, 1));
    await tester.pump();
    expect((controller.flings.single.$2 as FlingText).text, 'Room 204, 14:00');
    controller.sending.add(const SendProgress(sent: 1, total: 1, paths: 1));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
  });

  testWidgets('shared files are named in the header', (tester) async {
    await pumpOverlay(
      tester,
      content: const FlingFiles(['/a/notes.pdf', '/a/b.png', '/a/c.txt']),
    );
    expect(find.text('3 files'), findsNWidgets(2));
  });

  testWidgets('one file can be sent linked to a device that supports it', (
    tester,
  ) async {
    final linkedMac = EcDevice(
      id: mac.id,
      shortId: mac.shortId,
      name: mac.name,
      online: true,
      linkedFiles: true,
    );
    final (controller, gesture, results) = await pumpOverlay(
      tester,
      devices: [linkedMac],
      content: const FlingFiles(['/Users/test/Rapor.pdf']),
    );
    expect(find.text('Send linked'), findsOneWidget);
    await tester.tap(find.text('Send linked'));
    await tester.pump();
    expect(controller.sendLinked, isTrue);
    expect(results, isEmpty, reason: 'choosing does not close the layer');

    gesture.move(section(0, 1));
    await tester.pump(const Duration(milliseconds: 100));
    gesture.release(section(0, 1));
    await tester.pump();
    expect(controller.linkedSends.single.$2, '/Users/test/Rapor.pdf');
    expect(controller.flings, isEmpty);
    controller.sending.add(const SendProgress(sent: 1, total: 1, paths: 1));
    await tester.pumpAndSettle();
    await tester.pump(const Duration(seconds: 1));
    await tester.pumpAndSettle();
  });

  testWidgets('screenshots and older devices get no linked choice', (
    tester,
  ) async {
    await pumpOverlay(tester);
    expect(find.text('Send linked'), findsNothing);
  });
}
