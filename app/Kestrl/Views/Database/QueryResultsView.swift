import SwiftUI

/// The rows of the last query, one column per result column. Also shows the row count, duration and a progress view while it runs.
struct QueryResultsView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VStack(spacing: 0) {
            if store.isRunningQuery {
                ProgressView("Running…")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let result = store.queryResult {
                resultTable(result)
                footer(result)
            } else {
                EmptyPane(
                    "No results yet",
                    systemImage: "tablecells",
                    description: Text("Run a query to see its rows here.")
                )
            }
        }
    }

    private func resultTable(_ result: ShownQueryResult) -> some View {
        Table(result.rows) {
            TableColumnForEach(result.columns.indices, id: \.self) { index in
                TableColumn(result.columns[index]) { row in
                    QueryCellView(store: store, cell: row.cells[index])
                }
                .width(min: 120, ideal: 200)
            }
        }
        .overlay {
            if result.rows.isEmpty {
                EmptyPane("No rows", systemImage: "tablecells")
            }
        }
    }

    private func footer(_ result: ShownQueryResult) -> some View {
        HStack {
            Text("\(result.rowCount) \(result.rowCount == 1 ? "row" : "rows") · \(result.durationMs) ms")
                .monospacedDigit()
            Spacer()
        }
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
    }
}
