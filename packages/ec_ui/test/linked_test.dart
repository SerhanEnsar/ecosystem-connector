import 'package:ec_ui/ec_ui.dart';
import 'package:ec_ui/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

final now = DateTime(2026, 10, 19, 15);

final tablet = EcDevice(
  id: 't' * 64,
  shortId: '1111-2222',
  name: 'Tablet',
  online: true,
  linkedFiles: true,
);

EcLinkedFile file(
  String id,
  EcLinkedState state, {
  String name = 'Rapor.pdf',
}) => EcLinkedFile(
  id: id,
  name: name,
  path: '/Users/test/Documents/$name',
  devices: const ['Tablet'],
  state: state,
  size: 1200000,
  conflict: state == EcLinkedState.conflict
      ? EcLinkedConflict(
          mineDevice: 'Mac',
          mineAt: DateTime(2026, 10, 19, 14, 2),
          mineSize: 1200000,
          theirsDevice: 'Tablet',
          theirsAt: DateTime(2026, 10, 19, 14, 5),
          theirsSize: 1300000,
          theirsPath: '/home/linked/conflicts/$id-$name',
        )
      : null,
);

Future<void> pump(WidgetTester tester, Widget child) => tester.pumpWidget(
  MaterialApp(
    theme: ecLightTheme(),
    home: Scaffold(body: child),
  ),
);

void main() {
  test('times read as today, yesterday or a date', () {
    expect(formatWhen(DateTime(2026, 10, 19, 14, 2), now: now), 'today 14:02');
    expect(
      formatWhen(DateTime(2026, 10, 18, 9, 30), now: now),
      'yesterday 09:30',
    );
    expect(
      formatWhen(DateTime(2026, 10, 12, 18, 45), now: now),
      '2026-10-12 18:45',
    );
  });

  group('conflict card', () {
    testWidgets('shows both versions and keeps the chosen one', (tester) async {
      final controller = FakeController();
      final conflicted = file('a', EcLinkedState.conflict);
      await pump(
        tester,
        ConflictCard(controller: controller, file: conflicted, now: now),
      );
      expect(find.text('Rapor.pdf changed on two devices'), findsOneWidget);
      expect(find.text('Mac · today 14:02 · 1.2 MB'), findsOneWidget);
      expect(find.text('Tablet · today 14:05 · 1.3 MB'), findsOneWidget);

      await tester.tap(find.text("Keep Tablet's"));
      expect(controller.linkedCalls, ['resolve:a:theirs']);
      await tester.tap(find.text("Keep Mac's"));
      expect(controller.linkedCalls.last, 'resolve:a:mine');
    });

    testWidgets('open both opens the two versions and keeps the card', (
      tester,
    ) async {
      final controller = FakeController();
      final conflicted = file('a', EcLinkedState.conflict);
      await pump(
        tester,
        ConflictCard(controller: controller, file: conflicted, now: now),
      );
      await tester.tap(find.text('Open both'));
      expect(controller.opened, [
        '/Users/test/Documents/Rapor.pdf',
        '/home/linked/conflicts/a-Rapor.pdf',
      ]);
      expect(controller.linkedCalls, isEmpty);
      expect(find.text('Rapor.pdf changed on two devices'), findsOneWidget);
    });

    testWidgets('home shows it until it is closed', (tester) async {
      final controller = FakeController(devices: [tablet])
        ..linkedFiles = [file('a', EcLinkedState.conflict)];
      await tester.pumpWidget(
        MaterialApp(
          theme: ecLightTheme(),
          home: HomeScreen(controller: controller, platform: EcPlatform.macos),
        ),
      );
      expect(find.text('Rapor.pdf changed on two devices'), findsOneWidget);
      await tester.tap(find.bySemanticsLabel('Close').first);
      await tester.pump();
      expect(find.text('Rapor.pdf changed on two devices'), findsNothing);
    });
  });

  group('linked files screen', () {
    testWidgets('lists each file with its state', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [
          file('a', EcLinkedState.current, name: 'Rapor.pdf'),
          file('b', EcLinkedState.updating, name: 'Sunum.key'),
          file('c', EcLinkedState.conflict, name: 'Butce.xlsx'),
          file('d', EcLinkedState.missing, name: 'Notlar.md'),
        ];
      await pump(tester, LinkedFilesScreen(controller: controller));
      expect(find.text('Up to date · Tablet'), findsOneWidget);
      expect(find.text('Updating… · Tablet'), findsOneWidget);
      expect(find.text('Conflict (1.3 MB waiting) · Tablet'), findsOneWidget);
      expect(find.text('Missing · Tablet'), findsOneWidget);
    });

    testWidgets('a conflict can lose its waiting version', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('c', EcLinkedState.conflict)];
      await pump(tester, LinkedFilesScreen(controller: controller));
      await tester.tap(find.text('Rapor.pdf'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Delete waiting version'));
      expect(controller.linkedCalls, ['delete-conflict:c']);
    });

    testWidgets('leaving a link asks first', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('a', EcLinkedState.current)];
      await pump(tester, LinkedFilesScreen(controller: controller));
      await tester.tap(find.text('Rapor.pdf'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(TextButton, 'Leave link'));
      await tester.pumpAndSettle();
      expect(controller.linkedCalls, isEmpty);
      await tester.tap(find.widgetWithText(FilledButton, 'Leave link'));
      await tester.pumpAndSettle();
      expect(controller.linkedCalls, ['leave:a']);
    });

    testWidgets('a missing file can be located', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('d', EcLinkedState.missing)];
      await pump(tester, LinkedFilesScreen(controller: controller));
      await tester.tap(find.text('Rapor.pdf'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Show where it is'));
      expect(controller.linkedCalls, ['locate:d']);
    });

    testWidgets('the log is shown newest first', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('a', EcLinkedState.current)]
        ..log = [
          EcLinkedLog(
            at: DateTime(2026, 10, 12, 18, 45),
            device: 'Mac',
            text: 'changed',
          ),
        ];
      await pump(tester, LinkedFilesScreen(controller: controller));
      await tester.tap(find.text('Rapor.pdf'));
      await tester.pumpAndSettle();
      expect(find.text('2026-10-12 18:45 · Mac changed'), findsOneWidget);
    });

    testWidgets('a file missing on another device says where', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [
          EcLinkedFile(
            id: 'm',
            name: 'Makale.pdf',
            path: '/x/Makale.pdf',
            devices: const ['Galaxy'],
            state: EcLinkedState.current,
            size: 10,
            missingOn: const ['Galaxy'],
          ),
        ];
      await pump(tester, LinkedFilesScreen(controller: controller));
      expect(
        find.text('Up to date · missing on Galaxy · Galaxy'),
        findsOneWidget,
      );
    });

    testWidgets('check now refreshes', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('a', EcLinkedState.current)];
      await pump(tester, LinkedFilesScreen(controller: controller));
      await tester.tap(find.byTooltip('Check now'));
      await tester.pump();
      expect(controller.linkedCalls, ['refresh']);
      // Pulling the list down does the same.
      await tester.fling(find.text('Rapor.pdf'), const Offset(0, 400), 1000);
      await tester.pumpAndSettle();
      expect(controller.linkedCalls, ['refresh', 'refresh']);
    });

    testWidgets('settings lead to it', (tester) async {
      final controller = FakeController()
        ..linkedFiles = [file('a', EcLinkedState.current)];
      await pump(tester, SettingsScreen(controller: controller));
      expect(find.text('1 file'), findsOneWidget);
      await tester.tap(find.text('Linked files').last);
      await tester.pumpAndSettle();
      expect(find.text('Up to date · Tablet'), findsOneWidget);
    });
  });

  group('send linked', () {
    testWidgets('the share bar offers it for one file and a capable device', (
      tester,
    ) async {
      final old = EcDevice(
        id: 'o' * 64,
        shortId: '3333-4444',
        name: 'Old phone',
        online: true,
      );
      final controller = FakeController(devices: [tablet, old])
        ..pendingShare = ['/Users/test/Rapor.pdf'];
      await tester.pumpWidget(
        MaterialApp(
          theme: ecLightTheme(),
          home: HomeScreen(controller: controller, platform: EcPlatform.macos),
        ),
      );
      expect(find.text('Send linked'), findsOneWidget);
      await tester.tap(find.text('Send linked'));
      await tester.pump();
      expect(controller.sendLinked, isTrue);
      // An older device cannot be picked for a linked send.
      final oldChip = tester.widget<ActionChip>(
        find.widgetWithText(ActionChip, 'Old phone'),
      );
      expect(oldChip.onPressed, isNull);
      await tester.tap(find.widgetWithText(ActionChip, 'Tablet'));
      await tester.pump();
      expect(controller.linkedSends.single.$2, '/Users/test/Rapor.pdf');
      expect(controller.sends, isEmpty);
    });

    testWidgets('several files are sent plainly', (tester) async {
      final controller = FakeController(devices: [tablet])
        ..pendingShare = ['/a.pdf', '/b.pdf'];
      await tester.pumpWidget(
        MaterialApp(
          theme: ecLightTheme(),
          home: HomeScreen(controller: controller, platform: EcPlatform.macos),
        ),
      );
      expect(find.text('Send linked'), findsNothing);
    });
  });

  testWidgets('a linked arrival says it is kept up to date', (tester) async {
    await pump(
      tester,
      ArrivalCard(
        data: const ArrivalCardData(
          title: 'Rapor.pdf',
          from: 'Mac',
          detail: '1.2 MB',
          linked: true,
        ),
        onPrimary: () {},
        onClosed: () {},
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Linked, kept up to date · 1.2 MB'), findsOneWidget);
    expect(find.byIcon(Icons.link), findsOneWidget);
    // Let it leave so no timer is left running.
    await tester.pump(const Duration(seconds: 4));
    await tester.pumpAndSettle();
  });
}
