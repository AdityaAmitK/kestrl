import SwiftUI

/// The request outline as nested disclosure groups. Folders start expanded; a folder stays collapsed only if the user collapsed it.
struct RequestTreeRows: View {
    let nodes: [RequestNode]
    let store: WorkspaceStore

    var body: some View {
        ForEach(nodes) { node in
            if let request = node.request {
                HStack(spacing: 8) {
                    MethodBadge(method: request.method ?? "?")
                    Text(node.name)
                        .lineLimit(1)
                }
                .listRowSeparator(.hidden)
                .tag(request.path)
            } else {
                DisclosureGroup(isExpanded: expansion(for: node.id)) {
                    RequestTreeRows(nodes: node.children ?? [], store: store)
                } label: {
                    Label(node.name, systemImage: "folder")
                        .font(.system(size: 11, weight: .medium))
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private func expansion(for id: String) -> Binding<Bool> {
        Binding(
            get: { !store.collapsedFolders.contains(id) },
            set: { store.setFolder(id, expanded: $0) }
        )
    }
}
