import Cocoa
import FlutterMacOS

@main
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    return true
  }

  override func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool {
    return true
  }

  /// Files opened with the app and econnector://share addresses, both from
  /// "Share → ECOnnector" (ShareExtension/ShareViewController.swift).
  override func application(_ application: NSApplication, open urls: [URL]) {
    ShareInbox.shared.receive(urls)
    NSApp.activate(ignoringOtherApps: true)
  }
}

/// What was shared to the app, until Dart takes it. Dart asks with "take" when it
/// starts (a share can launch the app) and is told with "shared" afterwards.
final class ShareInbox {
  static let shared = ShareInbox()

  private var pending: [[String: Any]] = []
  private var channel: FlutterMethodChannel?
  // Dart listens once it has asked with "take"; before that, shares wait.
  private var listening = false

  func attach(_ messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(
      name: "io.github.serhanensar.ecosystemconnector/share",
      binaryMessenger: messenger
    )
    channel.setMethodCallHandler { [weak self] call, result in
      guard call.method == "take", let self else {
        result(FlutterMethodNotImplemented)
        return
      }
      result(self.pending)
      self.pending = []
      self.listening = true
    }
    self.channel = channel
  }

  func receive(_ urls: [URL]) {
    let files = urls.filter(\.isFileURL).map(\.path)
    var shares: [[String: Any]] = files.isEmpty ? [] : [["files": files]]
    for url in urls where url.scheme == "econnector" && url.host == "share" {
      let query = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems ?? []
      if let link = query.first(where: { $0.name == "url" })?.value {
        shares.append(["url": link])
      } else if let text = query.first(where: { $0.name == "text" })?.value {
        shares.append(["text": text])
      }
    }
    guard !shares.isEmpty else { return }
    if listening, let channel {
      for share in shares { channel.invokeMethod("shared", arguments: share) }
    } else {
      pending.append(contentsOf: shares)
    }
  }
}
