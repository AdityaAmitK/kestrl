import SwiftUI

/// Shown until a workspace is open.
struct WelcomeView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        ContentUnavailableView {
            Label("No workspace open", systemImage: "folder.badge.gearshape")
        } description: {
            Text("Open a folder that contains a .kestrl directory to send requests and browse history.")
        } actions: {
            Button("Open workspace…") {
                store.isShowingOpenPanel = true
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)
        }
    }
}
