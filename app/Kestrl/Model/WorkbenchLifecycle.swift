import AppKit

/// Keep unresolved drafts in memory, but confirm before quitting with unsaved edits.
@MainActor
final class WorkbenchLifecycle: NSObject, NSApplicationDelegate {
    weak var store: WorkspaceStore?
    weak var capture: CaptureStore?

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if store?.hasUnsavedDrafts == true {
            let alert = NSAlert()
            alert.messageText = "Quit with unsaved requests?"
            alert.informativeText = "Request drafts are kept in memory. Save the request files before quitting to keep your edits."
            alert.addButton(withTitle: "Keep working")
            alert.addButton(withTitle: "Discard drafts and quit")
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
