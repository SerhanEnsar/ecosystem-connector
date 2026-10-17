import 'package:flutter_local_notifications/flutter_local_notifications.dart';

/// System notifications for received files.
class Notifier {
  final _plugin = FlutterLocalNotificationsPlugin();
  var _ready = false;
  var _nextId = 1;

  static const _android = AndroidNotificationDetails(
    'received',
    'Received files',
    channelDescription: 'A notification when files from your devices arrive',
    importance: Importance.defaultImportance,
  );

  Future<void> init() async {
    try {
      _ready =
          await _plugin.initialize(
            settings: const InitializationSettings(
              android: AndroidInitializationSettings('@mipmap/ic_launcher'),
              macOS: DarwinInitializationSettings(),
              windows: WindowsInitializationSettings(
                appName: 'Ecosystem Connector',
                appUserModelId: 'SerhanEnsar.EcosystemConnector',
                guid: 'f5e2a5f9-f910-4342-892a-cdccc7ef4169',
              ),
            ),
          ) ??
          false;
    } catch (_) {
      // Notifications are a convenience; the app works without them.
      _ready = false;
    }
  }

  Future<void> received(String title, String body) async {
    if (!_ready) return;
    await _plugin.show(
      id: _nextId++,
      title: title,
      body: body,
      notificationDetails: const NotificationDetails(
        android: _android,
        macOS: DarwinNotificationDetails(),
      ),
    );
  }
}
