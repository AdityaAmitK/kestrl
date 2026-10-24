import SwiftUI

/// Request files grouped by folder. Each row has a method badge.
struct RequestsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedRequest) {
            OutlineGroup(store.requestTree, children: \.children) { node in
                if let request = node.request {
                    HStack(spacing: 8) {
                        MethodBadge(method: request.method ?? "?")
                        Text(node.name)
                            .lineLimit(1)
                    }
                    .tag(request.path)
                } else {
                    Label(node.name, systemImage: "folder")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .overlay {
            if store.requestTree.isEmpty {
                ContentUnavailableView(
                    "No requests",
                    systemImage: "doc.text",
                    description: Text("Add *.req.toml files to this workspace to see them here.")
                )
            }
        }
        .onChange(of: store.selectedRequest) { _, _ in
            Task { await store.requestSelectionChanged() }
        }
        .navigationTitle("Requests")
        .navigationSplitViewColumnWidth(min: 300, ideal: 420)
    }
}
