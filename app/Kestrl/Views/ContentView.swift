import SwiftUI

/// The main window: an empty state until a workspace is open, then the sidebar beside the open section.
/// Each section lays out its own panes, so History can give its table the full width.
struct ContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        Group {
            if store.workspace == nil {
                WelcomeView(store: store)
            } else {
                workspaceView
            }
        }
        .frame(minWidth: 960, minHeight: 600)
        .preferredColorScheme(store.appearance.colorScheme)
        .fileImporter(isPresented: $store.isShowingOpenPanel, allowedContentTypes: [.folder]) { result in
            switch result {
            case .success(let url):
                Task { await store.loadWorkspace(from: url) }
            case .failure(let error):
                store.report(error)
            }
        }
        .sheet(isPresented: $store.isShowingComparison) {
            CompareView(store: store)
        }
        .task {
            await store.restoreLastWorkspace()
        }
    }

    private var workspaceView: some View {
        NavigationSplitView {
            SidebarView(store: store)
        } detail: {
            sectionView
        }
        .navigationSplitViewStyle(.balanced)
        .navigationTitle(workspaceName)
        .navigationSubtitle(store.section.title)
        .toolbar {
            toolbarItems
        }
        .safeAreaInset(edge: .top, spacing: 0) {
            if let message = store.errorMessage {
                ErrorBanner(message: message) {
                    store.dismissError()
                }
            }
        }
        .task(id: store.section) {
            await store.sectionChanged()
        }
    }

    private var workspaceName: String {
        store.workspace.map { URL(fileURLWithPath: $0.root).lastPathComponent } ?? "Kestrl"
    }

    @ViewBuilder private var sectionView: some View {
        switch store.section {
        case .requests:
            AutosavingSplit(autosaveName: "kestrl.requests.list", axis: .horizontal, defaultFraction: 0.3) {
                RequestsContentView(store: store)
            } second: {
                RequestsDetailView(store: store)
            }
        case .history:
            AutosavingSplit(autosaveName: "kestrl.history.events", axis: .vertical, defaultFraction: 0.5) {
                HistoryContentView(store: store)
            } second: {
                HistoryDetailView(store: store)
            }
        case .environments:
            AutosavingSplit(autosaveName: "kestrl.environments.list", axis: .horizontal, defaultFraction: 0.35) {
                EnvironmentsContentView(store: store)
            } second: {
                EnvironmentDetailView(store: store)
            }
        case .secrets:
            AutosavingSplit(autosaveName: "kestrl.secrets.list", axis: .horizontal, defaultFraction: 0.35) {
                SecretsContentView(store: store)
            } second: {
                SecretDetailView(store: store)
            }
        }
    }

    @ToolbarContentBuilder private var toolbarItems: some ToolbarContent {
        if store.section == .requests {
            ToolbarItem {
                EnvironmentMenu(store: store)
            }
        }
    }
}
