import 'package:flutter_test/flutter_test.dart';

import 'package:ecosystem_connector/main.dart';

void main() {
  testWidgets('shows the empty device list on launch', (tester) async {
    await tester.pumpWidget(const EcosystemConnectorApp());

    expect(find.text('Ecosystem Connector'), findsOneWidget);
    expect(find.text('No devices yet'), findsOneWidget);
  });
}
