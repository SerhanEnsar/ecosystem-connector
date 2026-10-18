import 'package:ec_ui/ec_ui.dart';
import 'package:ec_ui/testing.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

Widget app(FakeController c) =>
    ECOnnectorApp(platform: EcPlatform.macos, controller: c);

String shownDir(WidgetTester tester) =>
    tester.widget<SelectableText>(find.byKey(const Key('receive-dir'))).data!;

void main() {
  testWidgets('the gear opens settings with the receive folder', (
    tester,
  ) async {
    final c = FakeController();
    await tester.pumpWidget(app(c));
    await tester.tap(find.byTooltip('Settings'));
    await tester.pumpAndSettle();
    expect(find.text('Received files'), findsOneWidget);
    expect(shownDir(tester), c.defaultReceiveDir);
    expect(find.text('Use the default folder'), findsNothing);
  });

  testWidgets('choosing a folder shows it, and the default can come back', (
    tester,
  ) async {
    final c = FakeController()..folderToPick = '/Volumes/Archive/Inbox';
    await tester.pumpWidget(MaterialApp(home: SettingsScreen(controller: c)));
    await tester.tap(find.text('Change folder…'));
    await tester.pumpAndSettle();
    expect(shownDir(tester), '/Volumes/Archive/Inbox');

    await tester.tap(find.text('Use the default folder'));
    await tester.pumpAndSettle();
    expect(shownDir(tester), c.defaultReceiveDir);
    expect(find.text('Use the default folder'), findsNothing);
  });

  testWidgets('cancelling the picker keeps the folder', (tester) async {
    final c = FakeController();
    await tester.pumpWidget(MaterialApp(home: SettingsScreen(controller: c)));
    await tester.tap(find.text('Change folder…'));
    await tester.pumpAndSettle();
    expect(shownDir(tester), c.defaultReceiveDir);
  });

  testWidgets('a fixed folder offers no change, fling setup is listed', (
    tester,
  ) async {
    final c = FakeController()
      ..canChooseReceiveDir = false
      ..flingSetup = const FlingSetup(
        canDrawOverlays: true,
        screenAccess: false,
        handleOn: true,
      );
    await tester.pumpWidget(MaterialApp(home: SettingsScreen(controller: c)));
    expect(find.text('Change folder…'), findsNothing);
    expect(find.text('Fling from the edge of the screen'), findsOneWidget);
  });
}
