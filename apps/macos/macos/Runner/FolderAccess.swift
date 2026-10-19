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

/// Keeps access to linked files across restarts: one security-scoped bookmark per
/// link id. A bookmark follows its file when it is moved or renamed on the same
/// volume, so restoring also tells where each file is now.
final class LinkedAccess {
  static let shared = LinkedAccess()

  private let key = "linkedBookmarks"
  private var open: [String: URL] = [:]

  /// Keeps access to the linked file `id` at `path`.
  func keep(id: String, path: String) {
    let url = URL(fileURLWithPath: path)
    guard let bookmark = try? url.bookmarkData(
      options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil
    ) else { return }
    // Access is granted through the URL resolved from the bookmark, not a plain path;
    // the access held so far is let go only once the new one is in place.
    var stale = false
    guard let scoped = try? URL(
      resolvingBookmarkData: bookmark, options: .withSecurityScope,
      relativeTo: nil, bookmarkDataIsStale: &stale
    ) else { return }
    var all = stored()
    all[id] = bookmark
    UserDefaults.standard.set(all, forKey: key)
    use(id, scoped)
  }

  /// Opens every kept file again and returns where each is now, by link id.
  func restore() -> [String: String] {
    var all = stored()
    var places: [String: String] = [:]
    for (id, bookmark) in all {
      var stale = false
      guard let url = try? URL(
        resolvingBookmarkData: bookmark, options: .withSecurityScope,
        relativeTo: nil, bookmarkDataIsStale: &stale
      ) else { continue }
      use(id, url)
      places[id] = url.path
      if stale, let fresh = try? url.bookmarkData(
        options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil
      ) {
        all[id] = fresh
      }
    }
    UserDefaults.standard.set(all, forKey: key)
    return places
  }

  private func stored() -> [String: Data] {
    UserDefaults.standard.dictionary(forKey: key) as? [String: Data] ?? [:]
  }

  private func use(_ id: String, _ url: URL) {
    let previous = open[id]
    if url.startAccessingSecurityScopedResource() {
      open[id] = url
      previous?.stopAccessingSecurityScopedResource()
    }
  }
}

/// Lets Dart keep and restore access to linked files, and move a version the user
/// chose not to keep to the Trash.
func registerLinkedChannel(_ messenger: FlutterBinaryMessenger) {
  let channel = FlutterMethodChannel(
    name: "io.github.serhanensar.ecosystemconnector/linked",
    binaryMessenger: messenger
  )
  channel.setMethodCallHandler { call, result in
    let args = call.arguments as? [String: Any] ?? [:]
    switch call.method {
    case "keep":
      if let id = args["id"] as? String, let path = args["path"] as? String {
        LinkedAccess.shared.keep(id: id, path: path)
      }
      result(nil)
    case "restore":
      result(LinkedAccess.shared.restore())
    case "trash":
      guard let path = args["path"] as? String else {
        result(false)
        return
      }
      NSWorkspace.shared.recycle([URL(fileURLWithPath: path)]) { _, error in
        DispatchQueue.main.async { result(error == nil) }
      }
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}
