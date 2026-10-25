import SwiftUI

/// Shown until a workspace is open.
struct WelcomeView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        ContentUnavailableView {
            Label("No workspace open", systemImage: "folder.badge.gearshape")
        } description: {
            Text("A workspace is a folder, usually your project repo, holding .kestrl configuration and requests/.")
        } actions: {
            Button("New workspace…") {
                store.isShowingNewPanel = true
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.defaultAction)

            Button("Open workspace…") {
                store.isShowingOpenPanel = true
            }
            .buttonStyle(.bordered)
        }
        .safeAreaInset(edge: .bottom) {
            if let message = store.errorMessage {
                ErrorBanner(message: message) { store.dismissError() }
            }
        }
    }
}
