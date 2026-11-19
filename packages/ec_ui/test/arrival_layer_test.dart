import 'dart:async';

import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

const page = ArrivalCardData(
  title: 'Flutter docs',
  from: 'Laptop',
  detail: 'flutter.dev',
  isPage: true,
);

/// Lets the drop fall and the card spread.
Future<void> settleIn(WidgetTester tester) async {
  for (var i = 0; i < 12; i++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

void main() {
  testWidgets('draws each arrival over the app and removes it when it leaves', (
    tester,
  ) async {
    final arrivals = StreamController<ArrivalEntry>.broadcast();
    addTearDown(arrivals.close);
    var opened = 0;
    await tester.pumpWidget(
      MaterialApp(
        builder: (context, child) =>
            ArrivalLayer(arrivals: arrivals.stream, child: child!),
        home: const Scaffold(body: Text('home')),
      ),
    );
    expect(find.text('Flutter docs'), findsNothing);

    arrivals.add(ArrivalEntry(data: page, onPrimary: () => opened++));
    await tester.pump();
    await settleIn(tester);
    expect(find.text('home'), findsOneWidget, reason: 'the app stays under');
    expect(find.text('Flutter docs'), findsOneWidget);

    // The card leaves on its own after its stay, and is removed from the layer.
    for (var i = 0; i < 40; i++) {
      await tester.pump(const Duration(milliseconds: 100));
    }
    await tester.pumpAndSettle();
    expect(find.text('Flutter docs'), findsNothing);
    expect(opened, 0);
  });

  testWidgets('the primary button runs the action and closes the card', (
    tester,
  ) async {
    final arrivals = StreamController<ArrivalEntry>.broadcast();
    addTearDown(arrivals.close);
    var opened = 0;
    await tester.pumpWidget(
      MaterialApp(
        builder: (context, child) =>
            ArrivalLayer(arrivals: arrivals.stream, child: child!),
        home: const Scaffold(body: SizedBox()),
      ),
    );
    arrivals.add(ArrivalEntry(data: page, onPrimary: () => opened++));
    await settleIn(tester);
    await tester.tap(find.text('Open in browser'));
    await tester.pumpAndSettle();
    expect(opened, 1);
  });
}
