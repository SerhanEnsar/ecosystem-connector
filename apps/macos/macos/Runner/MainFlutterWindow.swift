import Cocoa
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = self.frame
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)

    RegisterGeneratedPlugins(registry: flutterViewController)
    registerArrivalChannel(flutterViewController.engine.binaryMessenger)
    registerFolderChannel(flutterViewController.engine.binaryMessenger)
    registerLinkedChannel(flutterViewController.engine.binaryMessenger)
    ShareInbox.shared.attach(flutterViewController.engine.binaryMessenger)

    super.awakeFromNib()
  }
}

/// Lets Dart show the arrival card (ArrivalPanel.swift) for received files and pages.
private func registerArrivalChannel(_ messenger: FlutterBinaryMessenger) {
  let channel = FlutterMethodChannel(
    name: "io.github.serhanensar.ecosystemconnector/arrival",
    binaryMessenger: messenger
  )
  channel.setMethodCallHandler { call, result in
    guard call.method == "show", let args = call.arguments as? [String: Any] else {
      result(FlutterMethodNotImplemented)
      return
    }
    let from = args["from"] as? String ?? "a paired device"
    let title = args["title"] as? String ?? ""
    let size = (args["size"] as? NSNumber)?.int64Value ?? 0
    let kind: Arrival.Kind
    if let link = args["url"] as? String, let url = URL(string: link) {
      kind = .page(url)
    } else if let text = args["text"] as? String {
      kind = .text(text)
    } else if let paths = args["paths"] as? [String], paths.count > 1 {
      kind = .files(paths.map { URL(fileURLWithPath: $0) }, size: size)
    } else if let path = (args["paths"] as? [String])?.first {
      kind = .file(URL(fileURLWithPath: path), size: size)
    } else {
      result(FlutterError(code: "bad-args", message: "no path or url", details: nil))
      return
    }
    let linked = args["linked"] as? Bool ?? false
    ArrivalPresenter.shared.show(Arrival(kind: kind, title: title, from: from, linked: linked))
    result(nil)
  }
}
