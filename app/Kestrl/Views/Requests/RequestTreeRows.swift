import SwiftUI

/// The request outline, flattened to the rows that are visible. Collapsed folders' children are
/// simply not rendered, so a selected request inside a collapsed folder never lingers on screen
/// (nested `DisclosureGroup`s in a selectable `List` leave such rows behind).
struct RequestTreeRows: View {
    let nodes: [RequestNode]
    let store: WorkspaceStore
    /// While searching, every folder shows its matches regardless of collapse state.
    var isFiltering = false

    private struct VisibleRow: Identifiable {
        let node: RequestNode
        let depth: Int
        var id: String { node.id }
    }

    var body: some View {
        ForEach(visibleRows(nodes, depth: 0)) { row in
            if let request = row.node.request {
                requestRow(row.node, request: request, depth: row.depth)
                    .tag(request.path)
            } else {
                folderRow(row.node, depth: row.depth)
            }
        }
    }

    private func visibleRows(_ nodes: [RequestNode], depth: Int) -> [VisibleRow] {
        nodes.flatMap { node -> [VisibleRow] in
            let row = VisibleRow(node: node, depth: depth)
            guard node.request == nil, isExpanded(node) else { return [row] }
            return [row] + visibleRows(node.children ?? [], depth: depth + 1)
        }
    }

    private func isExpanded(_ node: RequestNode) -> Bool {
        isFiltering || !store.collapsedFolders.contains(node.id)
    }

    private func requestRow(_ node: RequestNode, request: RequestEntry, depth: Int) -> some View {
        HStack(spacing: 8) {
            MethodBadge(method: request.method ?? "?")
            Text(node.name)
                .lineLimit(1)
        }
        .padding(.leading, indent(depth) + 18)
        .listRowSeparator(.hidden)
    }

    private func folderRow(_ node: RequestNode, depth: Int) -> some View {
        let expanded = isExpanded(node)
        return HStack(spacing: 6) {
            Image(systemName: "chevron.right")
                .font(.system(size: 9, weight: .semibold))
                .foregroundStyle(.secondary)
                .rotationEffect(.degrees(expanded ? 90 : 0))
                .frame(width: 12)
            Image(systemName: expanded ? "folder" : "folder.fill")
                .foregroundStyle(.secondary)
            Text(node.name)
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(.secondary)
                .lineLimit(1)
            Spacer(minLength: 0)
        }
        .padding(.leading, indent(depth))
        .contentShape(Rectangle())
        .onTapGesture {
            withAnimation(.easeOut(duration: 0.12)) {
                store.setFolder(node.id, expanded: !expanded)
            }
        }
        .listRowSeparator(.hidden)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel("Folder \(node.name)")
        .accessibilityValue(expanded ? "Expanded" : "Collapsed")
        .accessibilityAction { store.setFolder(node.id, expanded: !expanded) }
    }

    private func indent(_ depth: Int) -> CGFloat { CGFloat(depth) * 14 }
}
