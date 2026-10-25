import SwiftUI

/// Kestrl for macOS: a workbench over the same workspace and history the CLI uses.
@main
struct KestrlApp: App {
    @NSApplicationDelegateAdaptor(WorkbenchLifecycle.self) private var lifecycle
    @State private var store = WorkspaceStore()
    @State private var capture = CaptureStore()

    var body: some Scene {
        WindowGroup("Kestrl") {
            ContentView(store: store, capture: capture)
                .onAppear { lifecycle.store = store }
                .onReceive(NotificationCenter.default.publisher(for: NSApplication.willTerminateNotification)) { _ in
                    capture.stopForQuit()
                }
        }
        .defaultSize(width: 1200, height: 760)
        .commands {
            KestrlCommands(store: store)
        }

        Settings {
            SettingsView(store: store)
        }
    }
}
