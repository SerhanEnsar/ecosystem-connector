import 'package:ec_ui/ec_ui.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('starts on the device list', (tester) async {
    await tester.pumpWidget(const EcosystemConnectorApp(platform: EcPlatform.macos));

    expect(find.text('No devices yet'), findsOneWidget);
  });
}
