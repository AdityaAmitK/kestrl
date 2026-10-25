import AppKit

/// Native editing with completion keys and fast previews for complete tokens.
final class VariableEditorTextView: NSTextView {
    private let hover = VariableHoverController()
    private var hoverAreas: [NSTrackingArea] = []
    private var previews: [(range: NSRange, name: String, text: String, isSecret: Bool)] = []
    var completions: VariableCompletionController?

    func updatePreviews(_ tokens: [(NSRange, String, String, Bool)]) {
        hover.dismiss()
        previews = tokens
        rebuildHoverAreas()
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        rebuildHoverAreas()
    }

    private func rebuildHoverAreas() {
        for area in hoverAreas { removeTrackingArea(area) }
        hoverAreas = []
        guard let layoutManager, let textContainer else { return }
        let origin = textContainerOrigin
        for (index, token) in previews.enumerated() where NSMaxRange(token.range) <= (string as NSString).length {
            let glyphs = layoutManager.glyphRange(forCharacterRange: token.range, actualCharacterRange: nil)
            layoutManager.enumerateEnclosingRects(forGlyphRange: glyphs, withinSelectedGlyphRange: NSRange(location: NSNotFound, length: 0), in: textContainer) { rect, _ in
                let area = NSTrackingArea(rect: rect.offsetBy(dx: origin.x, dy: origin.y), options: [.mouseEnteredAndExited, .activeInActiveApp], owner: self, userInfo: ["token": index])
                self.addTrackingArea(area)
                self.hoverAreas.append(area)
            }
        }
    }

    override func mouseEntered(with event: NSEvent) {
        guard completions?.isVisible != true,
              let area = event.trackingArea,
              let index = area.userInfo?["token"] as? Int,
              previews.indices.contains(index) else { return }
        let token = previews[index]
        hover.schedule(name: token.name, preview: token.text, isSecret: token.isSecret, range: token.range, rect: area.rect, in: self)
    }

    override func mouseExited(with event: NSEvent) { hover.dismiss() }

    override func keyDown(with event: NSEvent) {
        hover.dismiss()
        if completions?.handle(event) == true { return }
        super.keyDown(with: event)
    }

    override func resignFirstResponder() -> Bool {
        completions?.dismiss()
        hover.dismiss()
        return super.resignFirstResponder()
    }

    override func viewWillMove(toWindow newWindow: NSWindow?) {
        hover.dismiss()
        super.viewWillMove(toWindow: newWindow)
    }
}
