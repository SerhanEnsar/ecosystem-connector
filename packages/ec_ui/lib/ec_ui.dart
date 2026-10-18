/// Shared user interface of the Ecosystem Connector apps.
///
/// Each platform app in `apps/` depends on this package and provides an
/// [EcController]; this package itself contains no platform code.
library;

export 'src/app.dart';
export 'src/arrival/arrival_card.dart';
export 'src/controller.dart';
export 'src/fling/fling_overlay.dart';
export 'src/fling/fling_setup_card.dart';
export 'src/model.dart';
export 'src/platform.dart';
export 'src/screens/home_screen.dart';
export 'src/screens/pairing_sheet.dart';
export 'src/screens/send_sheet.dart';
export 'src/screens/settings_screen.dart';
export 'src/theme.dart';
