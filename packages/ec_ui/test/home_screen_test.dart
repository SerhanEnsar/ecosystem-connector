import 'package:ec_ui/ec_ui.dart';
import 'package:ec_ui/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

final tablet = EcDevice(
  id: 'a' * 64,
  shortId: '9703-2A5D',
  name: 'Galaxy Tab',
  online: true,
);
final phone = EcDevice(
  id: 'b' * 64,
  shortId: '1111-2222',
  name: 'Pixel',
  online: false,
);

Widget app(FakeController c) =>
    EcosystemConnectorApp(platform: EcPlatform.macos, controller: c);

void main() {
  testWidgets('empty state invites pairing', (tester) async {
    await tester.pumpWidget(app(FakeController()));
    expect(find.text('Test Mac'), findsOneWidget);
    expect(find.textContaining('No paired devices yet'), findsOneWidget);
    expect(find.text('Pair a device'), findsOneWidget);
  });

  testWidgets('lists devices with their state', (tester) async {
    await tester.pumpWidget(app(FakeController(devices: [tablet, phone])));
    expect(find.text('Galaxy Tab'), findsOneWidget);
    expect(find.textContaining('Online'), findsOneWidget);
    expect(find.textContaining('Offline'), findsOneWidget);
  });

  testWidgets('tapping an online device picks files and sends them', (
    tester,
  ) async {
    final c = FakeController(devices: [tablet])..picked = ['/tmp/a.pdf'];
    await tester.pumpWidget(app(c));
    await tester.tap(find.text('Galaxy Tab'));
    await tester.pump();
    expect(c.sends.single.$2, ['/tmp/a.pdf']);

    c.sending.add(
      const SendDone(
        files: 1,
        bytes: 2000000,
        elapsed: Duration(seconds: 1),
        paths: 2,
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Sent 1 file to Galaxy Tab'), findsOneWidget);
  });

  testWidgets('offline devices cannot be tapped', (tester) async {
    final c = FakeController(devices: [phone])..picked = ['/tmp/a.pdf'];
    await tester.pumpWidget(app(c));
    await tester.tap(find.text('Pixel'));
    await tester.pump();
    expect(c.sends, isEmpty);
  });

  testWidgets('receiving shows a progress banner', (tester) async {
    final c = FakeController(devices: [tablet]);
    await tester.pumpWidget(app(c));
    c.receiving = const ReceiveStatus(
      from: 'Galaxy Tab',
      files: 2,
      received: 5000000,
      total: 10000000,
    );
    await tester.pump();
    expect(find.text('Receiving 2 files from Galaxy Tab'), findsOneWidget);
    expect(find.text('5.0 MB of 10.0 MB'), findsOneWidget);
  });

  testWidgets('pairing: show code, compare, accept', (tester) async {
    final c = FakeController();
    await tester.pumpWidget(app(c));
    await tester.tap(find.text('Pair a device'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Show a code'));
    await tester.pump();

    c.pairing.add(const PairingInvite('ecpair:1?id=x&s=y&a=1.2.3.4:5'));
    await tester.pump();
    expect(find.text('Scan this code on your other device'), findsOneWidget);

    c.pairing.add(
      const PairingConfirm(
        peerName: 'Galaxy Tab',
        shortId: '9703-2A5D',
        code: '057 116',
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('057 116'), findsOneWidget);
    await tester.tap(find.text('Codes match'));
    await tester.pump();
    expect(c.answers, [true]);

    c.pairing.add(const PairingDone('Galaxy Tab'));
    await tester.pumpAndSettle();
    expect(find.text('Paired with Galaxy Tab'), findsOneWidget);
  });

  testWidgets('scan option only where a camera can scan', (tester) async {
    await tester.pumpWidget(app(FakeController(canScan: true)));
    await tester.tap(find.text('Pair a device'));
    await tester.pumpAndSettle();
    expect(find.text('Scan a code'), findsOneWidget);
  });

  testWidgets('files shared from another app go to the chosen device', (
    tester,
  ) async {
    final c = FakeController(devices: [tablet, phone])
      ..pendingShare = ['/cache/shared/photo.jpg'];
    await tester.pumpWidget(app(c));
    expect(find.text('Send 1 file to'), findsOneWidget);
    // Only online devices are offered.
    expect(find.widgetWithText(ActionChip, 'Pixel'), findsNothing);
    await tester.tap(find.widgetWithText(ActionChip, 'Galaxy Tab'));
    await tester.pump();
    expect(c.sends.single.$2, ['/cache/shared/photo.jpg']);
    expect(c.pendingShare, isEmpty);
  });

  testWidgets('a VPN shows a dismissible notice', (tester) async {
    await tester.pumpWidget(app(FakeController(vpnActive: true)));
    expect(find.text('A VPN is on'), findsOneWidget);
    await tester.tap(find.byTooltip('Dismiss'));
    await tester.pump();
    expect(find.text('A VPN is on'), findsNothing);
  });

  testWidgets('fling setup walks through what is missing', (tester) async {
    final c = FakeController()
      ..flingSetup = const FlingSetup(
        handleOn: false,
        canDrawOverlays: false,
        screenAccess: false,
      );
    await tester.pumpWidget(app(c));
    // Home only reminds; the steps are in settings.
    expect(find.text('Fling is not ready'), findsOneWidget);
    expect(find.textContaining('screen access is off'), findsOneWidget);
    await tester.tap(find.text('Set up'));
    await tester.pumpAndSettle();
    expect(find.text('Fling from the edge of the screen'), findsOneWidget);
    await tester.tap(find.text('Allow'));
    await tester.tap(find.text('Open settings'));
    expect(c.setupCalls, ['overlay', 'access']);

    c.flingSetup = const FlingSetup(
      handleOn: false,
      canDrawOverlays: true,
      screenAccess: true,
    );
    await tester.pump();
    await tester.tap(find.byType(Switch));
    expect(c.setupCalls.last, 'handle:true');

    c.flingSetup = const FlingSetup(
      handleOn: true,
      canDrawOverlays: true,
      screenAccess: true,
    );
    await tester.pump();
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.text('Fling is not ready'), findsNothing);
  });

  testWidgets('opening the app with a pairing link starts joining', (
    tester,
  ) async {
    final c = FakeController();
    await tester.pumpWidget(app(c));
    c.pendingInvite = 'ecpair:1?id=x&s=y&a=1.2.3.4:5';
    await tester.pump();
    await tester.pumpAndSettle();
    // Joining is underway; the sheet waits for the other device's code.
    expect(c.pendingInvite, isNull);
    expect(find.byType(PairingSheet), findsOneWidget);
    c.pairing.add(
      const PairingConfirm(
        peerName: 'Mac',
        shortId: '1ABE-79D4',
        code: '111 222',
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('111 222'), findsOneWidget);
  });

  test('formats byte counts', () {
    expect(formatBytes(999), '999 B');
    expect(formatBytes(314572800), '314.6 MB');
  });
}
