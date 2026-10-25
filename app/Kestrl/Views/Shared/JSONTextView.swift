import AppKit
import SwiftUI

/// A selectable, read-only monospaced text view. NSTextView stays responsive for multi-megabyte bodies, where a stack of SwiftUI `Text` rows would not.
struct JSONTextView: NSViewRepresentable {
    let text: String

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        guard let textView = scrollView.documentView as? NSTextView else { return scrollView }
        textView.isEditable = false
        textView.isSelectable = true
        textView.isRichText = false
        textView.allowsUndo = false
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.usesFindBar = true
        textView.isIncrementalSearchingEnabled = true
        textView.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        textView.textColor = .labelColor
        textView.backgroundColor = .textBackgroundColor
        textView.textContainerInset = NSSize(width: 12, height: 12)
        textView.setAccessibilityLabel("Body")
        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        guard let textView = scrollView.documentView as? NSTextView, textView.string != text else { return }
        let styled = NSMutableAttributedString(string: text, attributes: [
            .font: NSFont.monospacedSystemFont(ofSize: 12, weight: .regular),
            .foregroundColor: NSColor.labelColor
        ])
        // Keep large response bodies cheap; plain text stays selectable beyond this limit.
        if text.utf8.count <= 512_000, let tokens = try? NSRegularExpression(
            pattern: #""(?:\\.|[^"\\])*"|-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b|\b(?:true|false|null)\b"#
        ) {
            let source = text as NSString
            for match in tokens.matches(in: text, range: NSRange(location: 0, length: source.length)) {
                let token = source.substring(with: match.range)
                let end = match.range.location + match.range.length
                let remainder = source.substring(with: NSRange(location: end, length: min(64, source.length - end))).drop(while: { $0.isWhitespace })
                let color: NSColor
                if token.hasPrefix("\"") {
                    color = remainder.first == ":" ? .systemTeal : .systemGreen
                } else if token == "null" {
                    color = .secondaryLabelColor
                } else {
                    color = .systemOrange
                }
                styled.addAttribute(.foregroundColor, value: color, range: match.range)
            }
        }
        textView.textStorage?.setAttributedString(styled)
        textView.scrollToBeginningOfDocument(nil)
    }
}
