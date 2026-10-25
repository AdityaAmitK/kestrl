import AppKit
import SwiftUI

/// Plain-string binding with native selection, undo and variable completions.
struct VariableTextView: NSViewRepresentable {
    @Binding var text: String
    var variables: [VariableInfo]
    var label: String
    var environmentName: String?
    var singleLine = false
    var onSubmit: (() -> Void)?

    func makeCoordinator() -> VariableTextCoordinator { VariableTextCoordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        let view = VariableEditorTextView(frame: NSRect(x: 0, y: 0, width: 600, height: singleLine ? 28 : 200))
        view.isRichText = false
        view.allowsUndo = true
        view.isAutomaticQuoteSubstitutionEnabled = false
        view.isAutomaticDashSubstitutionEnabled = false
        view.isAutomaticTextReplacementEnabled = false
        view.isAutomaticSpellingCorrectionEnabled = false
        view.isContinuousSpellCheckingEnabled = false
        view.font = .monospacedSystemFont(ofSize: 12, weight: .regular)
        view.textColor = .textColor
        view.backgroundColor = .textBackgroundColor
        view.textContainerInset = NSSize(width: 6, height: singleLine ? 4 : 6)
        view.isVerticallyResizable = !singleLine
        view.isHorizontallyResizable = singleLine
        view.autoresizingMask = [.width]
        view.textContainer?.widthTracksTextView = !singleLine
        if singleLine { view.textContainer?.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: 24) }
        view.completions = context.coordinator.completions
        view.delegate = context.coordinator
        view.string = text
        view.setAccessibilityLabel(label)
        scroll.documentView = view
        scroll.hasVerticalScroller = !singleLine
        scroll.hasHorizontalScroller = false
        scroll.borderType = singleLine ? .bezelBorder : .noBorder
        context.coordinator.refresh(view)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        let coordinator = context.coordinator
        let changed = coordinator.parent.variables != variables
        coordinator.parent = self
        guard let view = scroll.documentView as? VariableEditorTextView else { return }
        if view.string != text {
            view.string = text
            view.undoManager?.removeAllActions()
            coordinator.refresh(view)
        } else if changed {
            coordinator.refresh(view)
            coordinator.completions.update(view, variables: variables, environmentName: environmentName)
        }
    }

    static func dismantleNSView(_ scroll: NSScrollView, coordinator: VariableTextCoordinator) {
        coordinator.highlightTask?.cancel()
        coordinator.completions.dismiss()
        (scroll.documentView as? NSTextView)?.delegate = nil
    }
}
