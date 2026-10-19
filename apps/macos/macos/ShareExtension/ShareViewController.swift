import AppKit
import UniformTypeIdentifiers

/// "Share → ECOnnector" on the Mac. Has no window of its own: it hands what is shared
/// to the app, which asks which device to send it to.
///
/// Files are opened with the app, so macOS lets the sandboxed app read them. A link or
/// text travels in an `econnector://share` address. Nothing is sent from here.
final class ShareViewController: NSViewController {
  /// Longest text handed over; matches MAX_TEXT_CHARS in the ec-fling crate.
  private let maxTextChars = 64 * 1024

  override func loadView() {
    view = NSView(frame: .zero)
  }

  override func viewDidLoad() {
    super.viewDidLoad()
    let providers = (extensionContext?.inputItems as? [NSExtensionItem] ?? [])
      .flatMap { $0.attachments ?? [] }
    collect(providers) { [weak self] files, link, text in
      self?.hand(files: files, link: link, text: text)
      self?.extensionContext?.completeRequest(returningItems: nil)
    }
  }

  /// Loads the shared files, the first web link and the first text, then calls `done`
  /// on the main queue.
  private func collect(
    _ providers: [NSItemProvider],
    done: @escaping ([URL], URL?, String?) -> Void
  ) {
    var files: [URL] = []
    var link: URL?
    var text: String?
    let group = DispatchGroup()
    let lock = NSLock()
    for provider in providers {
      if provider.hasItemConformingToTypeIdentifier(UTType.fileURL.identifier) {
        group.enter()
        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier) { item, _ in
          if let url = Self.url(from: item), url.isFileURL {
            lock.lock(); files.append(url); lock.unlock()
          }
          group.leave()
        }
      } else if provider.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
        group.enter()
        provider.loadItem(forTypeIdentifier: UTType.url.identifier) { item, _ in
          if let url = Self.url(from: item), ["http", "https"].contains(url.scheme?.lowercased()) {
            lock.lock(); if link == nil { link = url }; lock.unlock()
          }
          group.leave()
        }
      } else if provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
        group.enter()
        provider.loadItem(forTypeIdentifier: UTType.plainText.identifier) { item, _ in
          let value = (item as? String) ?? (item as? Data).flatMap { String(data: $0, encoding: .utf8) }
          lock.lock(); if text == nil { text = value }; lock.unlock()
          group.leave()
        }
      }
    }
    // Finder can offer the same file in more than one attachment; send it once.
    group.notify(queue: .main) {
      var seen = Set<String>()
      done(files.filter { seen.insert($0.standardizedFileURL.path).inserted }, link, text)
    }
  }

  private static func url(from item: NSSecureCoding?) -> URL? {
    if let url = item as? URL { return url }
    if let data = item as? Data { return URL(dataRepresentation: data, relativeTo: nil) }
    if let string = item as? String { return URL(string: string) }
    return nil
  }

  /// Opens the files, or the share address for a link or text, with the app.
  private func hand(files: [URL], link: URL?, text: String?) {
    // This extension lives in ECOnnector.app/Contents/PlugIns.
    let app = Bundle.main.bundleURL
      .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    var urls = files
    if urls.isEmpty {
      var share = URLComponents()
      share.scheme = "econnector"
      share.host = "share"
      if let link {
        share.queryItems = [URLQueryItem(name: "url", value: link.absoluteString)]
      } else if let text, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
        share.queryItems = [URLQueryItem(name: "text", value: String(text.prefix(maxTextChars)))]
      }
      if share.queryItems != nil, let url = share.url { urls = [url] }
    }
    guard !urls.isEmpty else { return }
    NSWorkspace.shared.open(urls, withApplicationAt: app, configuration: NSWorkspace.OpenConfiguration())
  }
}
