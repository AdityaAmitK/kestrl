import SwiftUI

/// One result cell. NULL is dimmed and italic. Long text is cut to a preview, with the full value on hover and in Copy value.
struct QueryCellView: View {
    let store: WorkspaceStore
    let cell: QueryCell

    var body: some View {
        Text(cell.preview)
            .font(.body.monospaced())
            .italic(cell.isNull)
            .foregroundStyle(cell.isNull ? .secondary : .primary)
            .lineLimit(1)
            .truncationMode(.tail)
            .help(cell.text)
            .contextMenu {
                Button("Copy value", systemImage: "doc.on.doc") {
                    store.copyToPasteboard(cell.text)
                }
            }
    }
}
