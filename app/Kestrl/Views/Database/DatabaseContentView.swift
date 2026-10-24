import SwiftUI

/// The workspace's connections. Each expands to its tables, which load when it opens.
struct DatabaseContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedDatabaseItem) {
            ForEach(store.connections, id: \.self) { name in
                DisclosureGroup(isExpanded: expansion(for: name)) {
                    ForEach(store.tablesByConnection[name] ?? [], id: \.self) { table in
                        Label(table.name, systemImage: "tablecells")
                            .tag(DatabaseSelection.table(
                                DatabaseTable(connection: name, schema: table.schema, name: table.name)
                            ))
                    }
                } label: {
                    Label(name, systemImage: "cylinder")
                        .tag(DatabaseSelection.connection(name))
                }
            }
        }
        .overlay {
            if store.connections.isEmpty {
                ContentUnavailableView(
                    "No connections",
                    systemImage: "cylinder.split.1x2",
                    description: Text("Add a file under .kestrl/connections/ to connect a database.")
                )
            }
        }
        .onChange(of: store.selectedDatabaseItem) { old, _ in
            Task { await store.databaseSelectionChanged(from: old) }
        }
    }

    private func expansion(for name: String) -> Binding<Bool> {
        Binding(
            get: { store.expandedConnections.contains(name) },
            set: { store.setConnectionExpanded(name, expanded: $0) }
        )
    }
}
