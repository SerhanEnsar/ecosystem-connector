/// The platform an app build targets.
///
/// Passed in by each app's `main.dart`, so shared code can adapt without
/// importing `dart:io`.
enum EcPlatform {
  android('Android'),
  macos('macOS'),
  windows('Windows');

  const EcPlatform(this.label);

  /// Human-readable platform name.
  final String label;

  /// Whether this is a desktop platform with a menu bar or tray icon.
  bool get isDesktop => this != EcPlatform.android;
}
