import SwiftUI

/// A searchable collection, with an explicit New request action.
struct RequestsContentView: View {
    @Bindable var store: WorkspaceStore
    @State private var search = ""

    private var entries: [RequestEntry] {
        (store.workspace?.requests ?? []).filter {
            search.isEmpty || ($0.name ?? $0.path).localizedCaseInsensitiveContains(search)
                || $0.path.localizedCaseInsensitiveContains(search)
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader("Collection", subtitle: "\(store.workspace?.requests.count ?? 0)") {
                Button { store.newRequest() } label: { Image(systemName: "plus") }
                    .buttonStyle(.plain).help("New request (⌘N)").accessibilityLabel("New request")
            }
            Divider()
            TextField("Find a request", text: $search)
                .textFieldStyle(.roundedBorder).controlSize(.small)
                .padding(8).accessibilityLabel("Find a request")
            List(selection: $store.selectedRequest) {
                RequestTreeRows(nodes: RequestNode.build(entries), store: store, isFiltering: !search.isEmpty)
            }
            .listStyle(.plain)
            .environment(\.defaultMinListRowHeight, 26)
            .overlay {
                if entries.isEmpty {
                    EmptyPane(search.isEmpty ? "No requests" : "No matches", systemImage: "doc.text",
                              description: Text(search.isEmpty ? "Create a request with + or open a collection." : "Try a different name."))
                }
            }
        }
        .onChange(of: store.selectedRequest) { _, _ in
            Task { await store.requestSelectionChanged() }
        }
    }
}
