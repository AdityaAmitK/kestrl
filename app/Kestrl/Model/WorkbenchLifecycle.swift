import AppKit

/// Keep unresolved drafts in memory, but confirm before quitting with unsaved edits.
@MainActor
final class WorkbenchLifecycle: NSObject, NSApplicationDelegate {
    weak var store: WorkspaceStore?
    weak var capture: CaptureStore?

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        let hasRequestDrafts = store?.hasUnsavedDrafts == true
        let hasEnvironmentEdits = store?.environmentEditor.hasChanges == true || store?.profileEditor.hasChanges == true
        if hasRequestDrafts || hasEnvironmentEdits {
            let alert = NSAlert()
            alert.messageText = "Quit with unsaved changes?"
            alert.informativeText = switch (hasRequestDrafts, hasEnvironmentEdits) {
            case (true, true): "You have unsaved request drafts and environment or profile edits. They are lost if you quit."
            case (true, false): "Request drafts are kept in memory. Save the request files before quitting to keep your edits."
            default: "Your environment or profile edits are not saved yet. They are lost if you quit."
            }
            alert.addButton(withTitle: "Keep working")
            alert.addButton(withTitle: "Discard and quit")
            alert.alertStyle = .warning
            guard alert.runModal() == .alertSecondButtonReturn else { return .terminateCancel }
        }
        if capture?.stopForQuit() == false {
            let alert = NSAlert()
            alert.messageText = "Capture is still stopping"
            alert.informativeText = "Kestrl will stay open until capture cleanup finishes. Wait for it to stop, then quit again."
            alert.addButton(withTitle: "Keep open")
            alert.runModal()
            return .terminateCancel
        }
        return .terminateNow
    }
}
