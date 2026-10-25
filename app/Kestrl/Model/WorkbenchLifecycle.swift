import AppKit

/// Keep unresolved drafts in memory, but confirm before quitting with unsaved edits.
@MainActor
final class WorkbenchLifecycle: NSObject, NSApplicationDelegate {
    weak var store: WorkspaceStore?

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard store?.hasUnsavedDrafts == true else { return .terminateNow }
        let alert = NSAlert()
        alert.messageText = "Quit with unsaved requests?"
        alert.informativeText = "Request drafts are kept in memory. Save the request files before quitting to keep your edits."
        alert.addButton(withTitle: "Keep working")
        alert.addButton(withTitle: "Discard drafts and quit")
        alert.alertStyle = .warning
        return alert.runModal() == .alertSecondButtonReturn ? .terminateNow : .terminateCancel
    }
}
