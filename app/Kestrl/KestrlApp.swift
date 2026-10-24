import SwiftUI

/// Kestrl for macOS: a workbench over the same workspace and history the CLI uses.
@main
struct KestrlApp: App {
    @State private var store = WorkspaceStore()

    var body: some Scene {
        WindowGroup("Kestrl") {
            ContentView(store: store)
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
