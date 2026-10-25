import SwiftUI

/// The SQL editor above the results, for the selected connection. Selecting a table opens its columns beside the results.
struct DatabaseDetailView: View {
    @Bindable var store: WorkspaceStore
    @State private var isShowingColumns = false

    var body: some View {
        if let connection = store.selectedConnection {
            VStack(spacing: 0) {
                header(for: connection)
                Divider()
                AutosavingSplit(autosaveName: "kestrl.database.query", axis: .vertical, defaultFraction: 0.35) {
                    editor
                } second: {
                    QueryResultsView(store: store)
                }
            }
            .inspector(isPresented: $isShowingColumns) {
                TableInspector(store: store)
            }
        } else {
            EmptyPane(
                "Select a connection",
                systemImage: "cylinder.split.1x2",
                description: Text("Pick a connection or one of its tables. Write SQL here and press ⌘↩ to run it.")
            )
        }
    }

    private func header(for connection: String) -> some View {
        HStack(spacing: 12) {
            Label(connection, systemImage: "cylinder")
                .font(.headline)
            WriteAccessBadge(isReadOnly: store.queryResult?.isReadOnly, environment: store.selectedEnvironment)

            Spacer()

            Button { isShowingColumns.toggle() } label: {
                Image(systemName: "sidebar.right")
            }
            .disabled(store.selectedTable == nil)
            .help("Show table columns")
            .accessibilityLabel("Show table columns")

            Button("Copy JSON", systemImage: "doc.on.doc") {
                store.copyQueryResultJSON()
            }
            .disabled(store.queryResult == nil)
            .help("Copy the result as JSON")

            Button {
                Task { await store.runQuery() }
            } label: {
                Label("Run", systemImage: "play.fill")
            }
            .buttonStyle(.borderedProminent)
            .keyboardShortcut(.return, modifiers: .command)
            .disabled(store.isRunningQuery || store.sql.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .help("Run the query (⌘↩)")
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private var editor: some View {
        TextEditor(text: $store.sql)
            .font(.body.monospaced())
            .padding(8)
            .overlay(alignment: .topLeading) {
                if store.sql.isEmpty {
                    Text("Write SQL. Press ⌘↩ to run it.")
                        .font(.body.monospaced())
                        .foregroundStyle(.tertiary)
                        .padding(13)
                        .allowsHitTesting(false)
                }
            }
            .accessibilityLabel("SQL")
    }

}
