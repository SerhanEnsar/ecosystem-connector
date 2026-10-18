import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

const file = ArrivalCardData(
  title: 'Screenshot 2026-10-18 00.42.49.jpg',
  from: "Serhan's Galaxy Tab",
  detail: '260 KB',
  canSaveElsewhere: true,
);

/// Lets the drop fall and the card spread (two animations in a row).
Future<void> settleIn(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

Future<List<String>> pumpCard(WidgetTester tester, ArrivalCardData data) async {
  final calls = <String>[];
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        backgroundColor: Colors.blueGrey,
        body: ArrivalCard(
          data: data,
          onPrimary: () => calls.add('primary'),
          onOpen: () => calls.add('open'),
          onSaveElsewhere: () => calls.add('save'),
          onClosed: () => calls.add('closed'),
        ),
      ),
    ),
  );
  return calls;
}

void main() {
  testWidgets('spreads from a drop into a card with the details', (
    tester,
  ) async {
    await pumpCard(tester, file);
    expect(find.text(file.title), findsNothing, reason: 'still a drop');
    await settleIn(tester);
    expect(find.text(file.title), findsOneWidget);
    expect(find.text("From Serhan's Galaxy Tab"), findsOneWidget);
    expect(find.text('Show in folder'), findsOneWidget);
    expect(find.text('Save to…'), findsOneWidget);
  });

  testWidgets('leaves on its own after a few seconds', (tester) async {
    final calls = await pumpCard(tester, file);
    await settleIn(tester);
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(calls, ['closed']);
  });

  testWidgets('stays while touched', (tester) async {
    final calls = await pumpCard(tester, file);
    await settleIn(tester);
    final gesture = await tester.startGesture(
      tester.getCenter(find.text(file.title)),
    );
    await tester.pump(const Duration(seconds: 5));
    expect(calls, isEmpty);
    await gesture.up();
    await tester.pump(const Duration(seconds: 3));
    await tester.pumpAndSettle();
    expect(calls, ['closed']);
  });

  testWidgets('show in folder acts, then the card leaves', (tester) async {
    final calls = await pumpCard(tester, file);
    await settleIn(tester);
    await tester.tap(find.text('Show in folder'));
    await tester.pumpAndSettle();
    expect(calls, ['primary', 'closed']);
  });

  testWidgets('× closes without acting', (tester) async {
    final calls = await pumpCard(tester, file);
    await settleIn(tester);
    await tester.tap(find.bySemanticsLabel('Close'));
    await tester.pumpAndSettle();
    expect(calls, ['closed']);
  });

  testWidgets('a page offers to open in the browser', (tester) async {
    await pumpCard(
      tester,
      const ArrivalCardData(
        title: 'QUIC - Wikipedia',
        from: 'Mac',
        detail: 'en.wikipedia.org',
        isPage: true,
      ),
    );
    await settleIn(tester);
    expect(find.text('Open in browser'), findsOneWidget);
    expect(find.text('Show in folder'), findsNothing);
    expect(find.text('Open'), findsNothing);
  });
}
