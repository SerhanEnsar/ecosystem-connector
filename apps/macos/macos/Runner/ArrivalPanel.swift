import AppKit
import QuickLookThumbnailing
import SwiftUI

/// Something that just arrived from another device.
struct Arrival: Identifiable {
  enum Kind {
    case file(URL, size: Int64)
    case files([URL], size: Int64)
    case page(URL)
  }

  let id = UUID()
  let kind: Kind
  let title: String
  let from: String
}

/// Shows arrivals at the top of the screen: a drop falls from the top edge and spreads
/// into a frosted card with a preview. The card stays a few seconds (longer while the
/// pointer is on it). Clicking shows the file in Finder or opens the page; × closes it.
final class ArrivalPresenter {
  static let shared = ArrivalPresenter()

  private var panels: [ArrivalPanel] = []

  func show(_ arrival: Arrival) {
    let panel = ArrivalPanel(arrival: arrival, slot: panels.count) { [weak self] closed in
      self?.panels.removeAll { $0 === closed }
      self?.restack()
    }
    panels.append(panel)
    panel.present()
  }

  private func restack() {
    for (index, panel) in panels.enumerated() { panel.move(to: index) }
  }
}

/// A borderless, non-activating, transparent panel holding one card. It never takes
/// focus from the app the user is working in.
final class ArrivalPanel: NSPanel {
  static let size = NSSize(width: 420, height: 168)
  private let onClose: (ArrivalPanel) -> Void

  init(arrival: Arrival, slot: Int, onClose: @escaping (ArrivalPanel) -> Void) {
    self.onClose = onClose
    super.init(
      contentRect: NSRect(origin: .zero, size: Self.size),
      styleMask: [.borderless, .nonactivatingPanel],
      backing: .buffered,
      defer: false
    )
    isOpaque = false
    backgroundColor = .clear
    hasShadow = false
    level = .statusBar
    collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary]
    isMovable = false
    hidesOnDeactivate = false

    let view = ArrivalView(arrival: arrival, model: ArrivalModel()) { [weak self] in self?.dismiss() }
    contentView = NSHostingView(rootView: view)
    move(to: slot, animated: false)
  }

  override var canBecomeKey: Bool { false }

  func present() {
    orderFrontRegardless()
  }

  /// Puts the panel in its row: the first one at the top of the screen with the
  /// pointer, the next ones below it.
  func move(to slot: Int, animated: Bool = true) {
    let screen = NSScreen.screens.first { NSMouseInRect(NSEvent.mouseLocation, $0.frame, false) }
      ?? NSScreen.main
    guard let visible = screen?.visibleFrame else { return }
    let origin = NSPoint(
      x: visible.midX - Self.size.width / 2,
      y: visible.maxY - Self.size.height - CGFloat(slot) * (Self.size.height - 24)
    )
    setFrame(NSRect(origin: origin, size: Self.size), display: true, animate: animated)
  }

  private func dismiss() {
    orderOut(nil)
    onClose(self)
  }
}

/// The card's animation state. A plain observable object rather than `@State`, so the
/// file also builds with the command line tools (where SwiftUI's macros are missing).
final class ArrivalModel: ObservableObject {
  /// 0 = a drop above the top edge, 1 = the full card.
  @Published var spread: CGFloat = 0
  @Published var dropped = false
  @Published var showContent = false
  @Published var hovering = false
  @Published var thumbnail: NSImage?
  @Published var closing = false
  /// Seconds left before the card leaves on its own.
  var remaining: Double = 3
}

/// The animated card.
struct ArrivalView: View {
  let arrival: Arrival
  @ObservedObject var model: ArrivalModel
  let close: () -> Void

  private let width: CGFloat = 380
  private let height: CGFloat = 124
  private let ticker = Timer.publish(every: 0.1, on: .main, in: .common).autoconnect()

  var body: some View {
    let w = 18 + (width - 18) * model.spread
    let h = 18 + (height - 18) * model.spread
    ZStack(alignment: .top) {
      ZStack {
        RoundedRectangle(cornerRadius: 9 + 13 * model.spread, style: .continuous)
          .fill(.ultraThinMaterial)
        RoundedRectangle(cornerRadius: 9 + 13 * model.spread, style: .continuous)
          .strokeBorder(Color.white.opacity(0.18), lineWidth: 1)
        if model.showContent { content.transition(.opacity) }
      }
      .frame(width: w, height: h)
      .shadow(color: .black.opacity(0.28 * model.spread), radius: 24, y: 10)
      .offset(y: model.dropped ? 12 : -30)
      .opacity(model.closing ? 0 : 1)
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    .contentShape(Rectangle())
    .onHover { model.hovering = $0 }
    .onAppear(perform: appear)
    .onReceive(ticker) { _ in tick() }
    .task { model.thumbnail = await Self.preview(for: arrival) }
  }

  private var content: some View {
    HStack(spacing: 14) {
      preview
        .frame(width: 84, height: 84)
        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
      VStack(alignment: .leading, spacing: 4) {
        Text(arrival.title)
          .font(.system(size: 14, weight: .semibold))
          .lineLimit(2)
        Text("From \(arrival.from)")
          .font(.system(size: 12))
          .foregroundStyle(.secondary)
          .lineLimit(1)
        Text(detail)
          .font(.system(size: 11))
          .foregroundStyle(.tertiary)
          .lineLimit(1)
        Spacer(minLength: 2)
        HStack(spacing: 8) {
          Button(action: primary) {
            Text(primaryLabel)
              .font(.system(size: 12, weight: .medium))
              .padding(.horizontal, 10)
              .padding(.vertical, 5)
              .background(Capsule().fill(Color.white.opacity(model.hovering ? 0.22 : 0.12)))
          }
          .buttonStyle(.plain)
          if case .file = arrival.kind {
            Button("Save to…", action: saveTo)
              .buttonStyle(.plain)
              .font(.system(size: 12))
              .foregroundStyle(.secondary)
          }
        }
      }
      Spacer(minLength: 0)
      VStack {
        Button(action: dismiss) {
          Image(systemName: "xmark")
            .font(.system(size: 11, weight: .bold))
            .frame(width: 22, height: 22)
            .background(Circle().fill(Color.white.opacity(0.12)))
        }
        .buttonStyle(.plain)
        .help("Close")
        Spacer()
      }
    }
    .padding(14)
    .contentShape(Rectangle())
    .onTapGesture(perform: primary)
  }

  @ViewBuilder private var preview: some View {
    if let thumbnail = model.thumbnail {
      Image(nsImage: thumbnail).resizable().aspectRatio(contentMode: .fill)
    } else {
      ZStack {
        Color.white.opacity(0.10)
        Image(systemName: icon).font(.system(size: 30, weight: .regular))
      }
    }
  }

  private var icon: String {
    switch arrival.kind {
    case .file: return "doc"
    case .files: return "doc.on.doc"
    case .page: return "globe"
    }
  }

  private var detail: String {
    switch arrival.kind {
    case .file(_, let size), .files(_, let size):
      return ByteCountFormatter.string(fromByteCount: size, countStyle: .file)
    case .page(let url):
      return url.host ?? url.absoluteString
    }
  }

  private var primaryLabel: String {
    switch arrival.kind {
    case .file, .files: return "Show in folder"
    case .page: return "Open in browser"
    }
  }

  private func appear() {
    withAnimation(.easeOut(duration: 0.35)) { model.dropped = true }
    withAnimation(.spring(response: 0.55, dampingFraction: 0.78).delay(0.3)) { model.spread = 1 }
    withAnimation(.easeIn(duration: 0.2).delay(0.62)) { model.showContent = true }
  }

  private func tick() {
    guard model.spread == 1, !model.hovering, !model.closing else { return }
    model.remaining -= 0.1
    if model.remaining <= 0 { dismiss() }
  }

  private func primary() {
    switch arrival.kind {
    case .file(let url, _): NSWorkspace.shared.activateFileViewerSelecting([url])
    case .files(let urls, _): NSWorkspace.shared.activateFileViewerSelecting(urls)
    case .page(let url): NSWorkspace.shared.open(url)
    }
    dismiss()
  }

  private func saveTo() {
    guard case .file(let url, _) = arrival.kind else { return }
    let panel = NSSavePanel()
    panel.nameFieldStringValue = url.lastPathComponent
    panel.canCreateDirectories = true
    model.remaining = .infinity
    if panel.runModal() == .OK, let target = panel.url {
      try? FileManager.default.moveItem(at: url, to: target)
      NSWorkspace.shared.activateFileViewerSelecting([target])
    }
    dismiss()
  }

  /// Shrinks back into a drop and leaves through the top edge.
  private func dismiss() {
    guard !model.closing else { return }
    withAnimation(.easeIn(duration: 0.15)) { model.showContent = false }
    withAnimation(.spring(response: 0.4, dampingFraction: 0.9).delay(0.1)) { model.spread = 0 }
    withAnimation(.easeIn(duration: 0.25).delay(0.4)) {
      model.dropped = false
      model.closing = true
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.7, execute: close)
  }

  static func preview(for arrival: Arrival) async -> NSImage? {
    guard case .file(let url, _) = arrival.kind else { return nil }
    let request = QLThumbnailGenerator.Request(
      fileAt: url,
      size: CGSize(width: 168, height: 168),
      scale: NSScreen.main?.backingScaleFactor ?? 2,
      representationTypes: .thumbnail
    )
    return try? await QLThumbnailGenerator.shared.generateBestRepresentation(for: request).nsImage
  }
}
