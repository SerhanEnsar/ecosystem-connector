import 'package:ec_ui/ec_ui.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('shows the empty state and the platform', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(home: DevicesScreen(platform: EcPlatform.windows)),
    );

    expect(find.text('No devices yet'), findsOneWidget);
    expect(find.text('Windows build'), findsOneWidget);
  });

  test('only Android is not a desktop platform', () {
    expect(EcPlatform.values.where((p) => !p.isDesktop), [EcPlatform.android]);
  });
}
