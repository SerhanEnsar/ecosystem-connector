import 'package:ec_ui/ec_ui.dart';
import 'package:ec_ui/testing.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('starts on the device list', (tester) async {
    await tester.pumpWidget(
      ECOnnectorApp(platform: EcPlatform.windows, controller: FakeController()),
    );

    expect(find.textContaining('No paired devices yet'), findsOneWidget);
  });
}
