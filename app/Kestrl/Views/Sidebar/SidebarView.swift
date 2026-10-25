import SwiftUI

/// Stable navigation with workspace configuration separate from the daily workbench.
struct SidebarView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VStack(spacing: 0) {
            List(selection: $store.section) {
                Section("Workbench") {
                    ForEach([SidebarSection.requests, .history, .capture, .logs, .database]) { section in
                        Label(section.title, systemImage: section.symbol).tag(section)
                    }
                }
                Section("Workspace") {
                    ForEach([SidebarSection.environments, .profiles, .secrets]) { section in
                        Label(section.title, systemImage: section.symbol).tag(section)
                    }
                }
            }
            .listStyle(.sidebar)
            .environment(\.defaultMinListRowHeight, 28)

            Divider()
            HStack(spacing: 8) {
                Image(systemName: "folder").foregroundStyle(.secondary)
                Text(store.workspace.map { URL(fileURLWithPath: $0.root).lastPathComponent } ?? "Workspace")
                    .lineLimit(1).truncationMode(.middle)
                Spacer(minLength: 0)
                Button { store.isShowingOpenPanel = true } label: {
                    Image(systemName: "folder.badge.plus")
                }
                .buttonStyle(.plain)
                .help("Open workspace (⌘O)")
                .accessibilityLabel("Open workspace")
            }
            .font(.system(size: 11))
            .padding(12)
        }
        .navigationSplitViewColumnWidth(min: 170, ideal: 180, max: 220)
    }
}
