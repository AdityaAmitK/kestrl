import SwiftUI

/// The main window: an empty state until a workspace is open, then the three-column split view.
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
        } content: {
            contentColumn
        } detail: {
            detailColumn
        }
        .navigationSplitViewStyle(.balanced)
        .navigationTitle(workspaceName)
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

    @ViewBuilder private var contentColumn: some View {
        switch store.section {
        case .requests: RequestsContentView(store: store)
        case .history: HistoryContentView(store: store)
        case .environments: EnvironmentsContentView(store: store)
        case .secrets: SecretsContentView(store: store)
        }
    }

    @ViewBuilder private var detailColumn: some View {
        switch store.section {
        case .requests: RequestsDetailView(store: store)
        case .history: HistoryDetailView(store: store)
        case .environments: EnvironmentDetailView(store: store)
        case .secrets: SecretDetailView(store: store)
        }
    }

    @ToolbarContentBuilder private var toolbarItems: some ToolbarContent {
        ToolbarItem {
            EnvironmentMenu(store: store)
        }
        if store.section == .requests {
            ToolbarItem {
                Button {
                    Task { await store.sendSelectedRequest() }
                } label: {
                    Label("Send", systemImage: "paperplane.fill")
                }
                .help("Send the selected request (⌘↩)")
                .disabled(store.selectedRequest == nil || store.isSending)
            }
        }
        if store.section == .requests || store.section == .history {
            ToolbarItem {
                CopyJSONButton(store: store)
            }
        }
    }
}
