import AppKit

/// A short hover delay and a readable preview, independent of the system tooltip timer.
@MainActor
final class VariableHoverController {
    private let panel = VariableCompletionPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    private var pending: Task<Void, Never>?
    private var currentRange: NSRange?

    func schedule(name: String, preview: String, isSecret: Bool, range: NSRange, rect: NSRect, in view: NSView) {
        guard currentRange != range else { return }
        dismiss()
        currentRange = range
        pending = Task { [weak self, weak view] in
            try? await Task.sleep(for: .milliseconds(180))
            guard !Task.isCancelled, let self, let view, let window = view.window else { return }
            show(name: name, preview: preview, isSecret: isSecret, rect: rect, view: view, window: window)
        }
    }

    func dismiss() {
        pending?.cancel()
        currentRange = nil
        panel.parent?.removeChildWindow(panel)
        panel.orderOut(nil)
    }

    private func show(name: String, preview: String, isSecret: Bool, rect: NSRect, view: NSView, window: NSWindow) {
        let width: CGFloat = 360
        let font = NSFont.systemFont(ofSize: 12)
        let text = String(preview.prefix(500)) + (preview.count > 500 ? "…" : "")
        let measured = (text as NSString).boundingRect(with: NSSize(width: width - 24, height: CGFloat.greatestFiniteMagnitude), options: [.usesLineFragmentOrigin, .usesFontLeading], attributes: [.font: font])
        let textHeight = min(CGFloat(76), max(CGFloat(16), ceil(measured.height)))
        let height = textHeight + 42
        let surface = NSVisualEffectView(frame: NSRect(x: 0, y: 0, width: width, height: height))
        surface.material = .popover
        surface.state = .active
        surface.wantsLayer = true
        surface.layer?.cornerRadius = 9
        surface.layer?.masksToBounds = true
        let icon = NSImageView(frame: NSRect(x: 12, y: height - 26, width: 14, height: 16))
        icon.image = NSImage(systemSymbolName: isSecret ? "key.fill" : "curlybraces", accessibilityDescription: isSecret ? "Secret" : "Variable")
        icon.contentTintColor = isSecret ? .systemPurple : .secondaryLabelColor
        let title = NSTextField(labelWithString: "{{\(name)}}")
        title.font = .monospacedSystemFont(ofSize: 12, weight: .semibold)
        title.lineBreakMode = .byTruncatingMiddle
        title.frame = NSRect(x: 34, y: height - 26, width: width - 46, height: 16)
        let value = NSTextField(wrappingLabelWithString: text)
        value.font = font
        value.textColor = .secondaryLabelColor
        value.maximumNumberOfLines = 4
        value.frame = NSRect(x: 12, y: 10, width: width - 24, height: textHeight)
        for child in [icon, title, value] as [NSView] { surface.addSubview(child) }
        panel.contentView = surface
        panel.appearance = view.effectiveAppearance
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .popUpMenu
        panel.hidesOnDeactivate = true
        panel.isReleasedWhenClosed = false
        let token = window.convertToScreen(view.convert(rect, to: nil))
        let screen = window.screen?.visibleFrame ?? window.frame
        let x = min(max(token.minX, screen.minX), screen.maxX - width)
        let y = token.minY - height - 5 >= screen.minY ? token.minY - height - 5 : token.maxY + 5
        panel.setFrame(NSRect(x: x, y: y, width: width, height: height), display: true)
        window.addChildWindow(panel, ordered: .above)
        panel.orderFront(nil)
    }
}
