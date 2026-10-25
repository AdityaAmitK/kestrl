import SwiftUI

/// The main window: an empty state until a workspace is open, then the sidebar beside the open section.
/// Each section lays out its own panes, so History can give its table the full width.
struct ContentView: View {
    @Bindable var store: WorkspaceStore
    @Bindable var capture: CaptureStore

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
        .onChange(of: store.workspace?.root) { _, _ in
            capture.stopForWorkspaceChange()
        }
        .onChange(of: store.isShowingOpenPanel) { _, isShowing in
            guard isShowing else { return }
            store.isShowingOpenPanel = false
            Task { await store.chooseWorkspace() }
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
        .navigationTitle(store.section.title)
        .navigationSubtitle(workspaceName)
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
        .onChange(of: store.selectedEnvironment) { _, _ in
            Task { await store.environmentChanged() }
        }
    }

    private var workspaceName: String {
        store.workspace.map { URL(fileURLWithPath: $0.root).lastPathComponent } ?? "Kestrl"
    }

    @ViewBuilder private var sectionView: some View {
        switch store.section {
        case .requests:
            // A new autosave name drops the divider people dragged out under the old widths.
            AutosavingSplit(autosaveName: "kestrl.requests.list.v5", axis: .horizontal, firstPaneWidth: .content) {
                RequestsContentView(store: store)
            } second: {
                RequestsDetailView(store: store)
            }
        case .history:
            AutosavingSplit(autosaveName: "kestrl.history.events", axis: .vertical) {
                HistoryContentView(store: store)
            } second: {
                HistoryDetailView(store: store)
            }
        case .capture:
            AutosavingSplit(autosaveName: "kestrl.capture.exchanges.v1", axis: .horizontal, firstPaneWidth: PaneWidth(minimum: 300, ideal: 340, maximum: 460)) {
                CaptureContentView(capture: capture, workspaceRoot: store.workspace?.root)
            } second: {
                CaptureDetailView(capture: capture)
            }
        case .logs:
            AutosavingSplit(autosaveName: "kestrl.logs.services.v4", axis: .horizontal, firstPaneWidth: .content) {
                LogsContentView(store: store)
            } second: {
                LogsDetailView(store: store)
            }
        case .database:
            AutosavingSplit(autosaveName: "kestrl.database.list.v4", axis: .horizontal, firstPaneWidth: .content) {
                DatabaseContentView(store: store)
            } second: {
                DatabaseDetailView(store: store)
            }
        case .environments:
            AutosavingSplit(autosaveName: "kestrl.environments.list.v4", axis: .horizontal, firstPaneWidth: .content) {
                EnvironmentsContentView(store: store)
            } second: {
                EnvironmentDetailView(store: store)
            }
        case .secrets:
            AutosavingSplit(autosaveName: "kestrl.secrets.list.v4", axis: .horizontal, firstPaneWidth: .content) {
                SecretsContentView(store: store)
            } second: {
                SecretDetailView(store: store)
            }
        }
    }

    @ToolbarContentBuilder private var toolbarItems: some ToolbarContent {
        if store.section == .requests || store.section == .database {
            ToolbarItem {
                EnvironmentMenu(store: store)
            }
        }
    }
}
