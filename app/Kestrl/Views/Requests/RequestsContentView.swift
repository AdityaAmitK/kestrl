import SwiftUI

/// Request files grouped by folder. Each row has a method badge.
struct RequestsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedRequest) {
            RequestTreeRows(nodes: store.requestTree, store: store)
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
    }
}
