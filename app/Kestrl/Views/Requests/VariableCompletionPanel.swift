import AppKit

/// Completion clicks leave the text editor as the key window's first responder.
final class VariableCompletionPanel: NSPanel {
    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}
