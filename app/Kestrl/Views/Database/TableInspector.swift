import SwiftUI

/// The columns of the selected table, from `db_describe`.
struct TableInspector: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        if let table = store.selectedTable {
            VStack(alignment: .leading, spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(table.name)
                        .font(.headline)
                    Text(table.schema)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .padding(.horizontal, 12)
                .padding(.top, 12)

                Table(store.tableColumns) {
                    TableColumn("Column") { column in
                        Text(column.name)
                            .font(.body.monospaced())
                    }
                    TableColumn("Type") { column in
                        Text(column.dataType)
                            .font(.body.monospaced())
                            .foregroundStyle(.secondary)
                    }
                    TableColumn("Nullable") { column in
                        Text(column.isNullable ? "Yes" : "No")
                            .foregroundStyle(.secondary)
                    }
                    .width(min: 64, ideal: 72)
                }
            }
            .inspectorColumnWidth(min: 280, ideal: 340, max: 440)
        }
    }
}
