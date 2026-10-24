import SwiftUI

/// The SQL editor above the results, for the selected connection. Selecting a table opens its columns beside the results.
struct DatabaseDetailView: View {
    @Bindable var store: WorkspaceStore

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
            .inspector(isPresented: inspectorBinding) {
                TableInspector(store: store)
            }
        } else {
            ContentUnavailableView(
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
            Text(store.selectedEnvironment.map { "Environment \($0)" } ?? "No environment")
                .foregroundStyle(.secondary)
            WriteAccessBadge(isReadOnly: store.queryResult?.isReadOnly, environment: store.selectedEnvironment)

            Spacer()

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
            .keyboardShortcut(.return, modifiers: .command)
            .disabled(store.isRunningQuery || store.sql.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .help("Run the query (⌘↩)")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
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

    private var inspectorBinding: Binding<Bool> {
        Binding(
            get: { store.selectedTable != nil },
            set: { isShown in
                if !isShown { store.closeTableInspector() }
            }
        )
    }
}
