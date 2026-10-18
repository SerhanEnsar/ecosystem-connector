import AppKit
import FlutterMacOS

/// Keeps access to the folder the user chose for received files.
///
/// The app is sandboxed: it may write to ~/Downloads, and to a folder the user picks
/// in an open panel, but only until it quits. A security-scoped bookmark saved for that
/// folder lets the app open it again after a restart. Only the one chosen folder is
/// kept; choosing another or going back to the default lets go of it.
final class FolderAccess {
  static let shared = FolderAccess()

  private let key = "receiveFolderBookmark"
  private var current: URL?

  /// Shows an open panel for a folder. Returns its path, or nil if cancelled.
  func choose() -> String? {
    let panel = NSOpenPanel()
    panel.canChooseDirectories = true
    panel.canChooseFiles = false
    panel.canCreateDirectories = true
    panel.allowsMultipleSelection = false
    panel.prompt = "Save received files here"
    guard panel.runModal() == .OK, let url = panel.url else { return nil }
    guard let bookmark = try? url.bookmarkData(
      options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil
    ) else { return nil }
    UserDefaults.standard.set(bookmark, forKey: key)
    use(url)
    return url.path
  }

  /// Opens the folder chosen in an earlier run again. Call before the core starts.
  func restore() {
    guard let bookmark = UserDefaults.standard.data(forKey: key) else { return }
    var stale = false
    guard let url = try? URL(
      resolvingBookmarkData: bookmark, options: .withSecurityScope,
      relativeTo: nil, bookmarkDataIsStale: &stale
    ) else {
      // The folder is gone; the core falls back to the default folder.
      forget()
      return
    }
    use(url)
    if stale, let fresh = try? url.bookmarkData(
      options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil
    ) {
      UserDefaults.standard.set(fresh, forKey: key)
    }
  }

  /// Lets go of the chosen folder.
  func forget() {
    current?.stopAccessingSecurityScopedResource()
    current = nil
    UserDefaults.standard.removeObject(forKey: key)
  }

  private func use(_ url: URL) {
    current?.stopAccessingSecurityScopedResource()
    current = url.startAccessingSecurityScopedResource() ? url : nil
  }
}

/// Lets Dart choose, restore and forget the receive folder.
func registerFolderChannel(_ messenger: FlutterBinaryMessenger) {
  let channel = FlutterMethodChannel(
    name: "io.github.serhanensar.ecosystemconnector/folder",
    binaryMessenger: messenger
  )
  channel.setMethodCallHandler { call, result in
    switch call.method {
    case "choose": result(FolderAccess.shared.choose())
    case "restore": FolderAccess.shared.restore(); result(nil)
    case "forget": FolderAccess.shared.forget(); result(nil)
    default: result(FlutterMethodNotImplemented)
    }
  }
}
