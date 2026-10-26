import AppKit

/// Debounces attribute edits without replacing text storage or disturbing undo.
@MainActor
final class VariableTextCoordinator: NSObject, NSTextViewDelegate {
    var parent: VariableTextView
    var highlightTask: Task<Void, Never>?
    let completions = VariableCompletionController()
    private let tokens = try? NSRegularExpression(pattern: #"\{\{([^{}]+)\}\}"#)

    init(_ parent: VariableTextView) { self.parent = parent }

    func textDidChange(_ notification: Notification) {
        guard let view = notification.object as? VariableEditorTextView else { return }
        parent.text = view.string
        highlightTask?.cancel()
        highlightTask = Task { [weak self, weak view] in
            try? await Task.sleep(for: .milliseconds(100))
            guard !Task.isCancelled, let self, let view, !view.hasMarkedText() else { return }
            refresh(view)
        }
        completions.update(view, variables: parent.variables, environmentName: parent.environmentName)
    }

    func textViewDidChangeSelection(_ notification: Notification) {
        guard let view = notification.object as? VariableEditorTextView else { return }
        completions.update(view, variables: parent.variables, environmentName: parent.environmentName)
    }

    func textView(_ textView: NSTextView, shouldChangeTextIn affectedCharRange: NSRange, replacementString: String?) -> Bool {
        guard parent.singleLine, let replacementString else { return true }
        return !replacementString.contains(where: { $0.isNewline })
    }

    func textView(_ textView: NSTextView, doCommandBy selector: Selector) -> Bool {
        if parent.singleLine, selector == #selector(NSResponder.insertNewline(_:)) {
            parent.onSubmit?()
            return true
        }
        return false
    }

    func refresh(_ view: NSTextView) {
        guard let storage = view.textStorage, !view.hasMarkedText() else { return }
        let full = NSRange(location: 0, length: storage.length)
        let font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
        storage.beginEditing()
        storage.setAttributes([.font: font, .foregroundColor: NSColor.textColor], range: full)
        var descriptions: [String] = []
        var previews: [(NSRange, String, String, Bool)] = []
        var hasUndefinedVariable = false
        let catalog = Dictionary(parent.variables.map { ($0.name, $0) }, uniquingKeysWith: { _, last in last })
        for match in tokens?.matches(in: view.string, range: full) ?? [] {
            let name = (view.string as NSString).substring(with: match.range(at: 1)).trimmingCharacters(in: .whitespacesAndNewlines)
            let info = catalog[name]
            let secret = name.hasPrefix("secret:") || info?.isSecret == true
            let tooltip: String
            let color: NSColor
            var attributes: [NSAttributedString.Key: Any] = [.font: NSFont.monospacedSystemFont(ofSize: 12, weight: .semibold)]
            if secret {
                color = .systemPurple
                if name.hasPrefix("secret:") {
                    tooltip = info?.isSet == true ? "Secret — set" : "Secret — not set, run `kestrl secret set \(name.dropFirst(7))`"
                } else {
                    tooltip = "\(info?.source ?? "Variable"): secret — \(info?.isSet == true ? "set" : "not set"); value hidden"
                }
                attributes[.underlineStyle] = NSUnderlineStyle.single.rawValue
            } else if let info {
                color = .controlAccentColor
                let source = info.source == "environment" ? (parent.environmentName ?? "environment") : "workspace"
                tooltip = "\(source): \(info.value)"
            } else {
                hasUndefinedVariable = true
                color = .systemOrange
                tooltip = "\(name): undefined variable"
                attributes[.underlineStyle] = NSUnderlineStyle.patternDot.rawValue | NSUnderlineStyle.single.rawValue
            }
            attributes[.foregroundColor] = info == nil && !secret ? NSColor.textColor : color
            attributes[.underlineColor] = color
            attributes[.backgroundColor] = color.withAlphaComponent(0.12)
            storage.addAttributes(attributes, range: match.range)
            descriptions.append(tooltip)
            previews.append((match.range, name, tooltip, secret))
        }
        storage.endEditing()
        (view as? VariableEditorTextView)?.updatePreviews(previews)
        view.typingAttributes = [.font: font, .foregroundColor: NSColor.textColor]
        view.setAccessibilityLabel(parent.label + (hasUndefinedVariable ? ", undefined variable" : ""))
        view.setAccessibilityHelp(descriptions.joined(separator: ". "))
    }
}
