import SwiftUI

/// The workspace's connections, each expanding to its tables, with create / edit / delete.
/// The outline is flattened to visible rows so a collapsed connection never leaves a selected
/// table on screen.
struct DatabaseContentView: View {
    @Bindable var store: WorkspaceStore
    @State private var editing: ConnectionEdit?
    @State private var pendingDeletion: String?

    private struct ConnectionEdit: Identifiable {
        let original: String?
        var id: String { original ?? "<new>" }
    }

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader("Connections", subtitle: store.connections.isEmpty ? nil : "\(store.connections.count)") {
                Button {
                    editing = ConnectionEdit(original: nil)
                } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .disabled(store.workspace?.envs.isEmpty ?? true)
                .help(store.workspace?.envs.isEmpty ?? true ? "Create an environment first" : "New connection")
                .accessibilityLabel("New connection")
            }
            Divider()
            List(selection: $store.selectedDatabaseItem) {
                ForEach(store.connections, id: \.self) { name in
                    connectionRow(name)
                        .tag(DatabaseSelection.connection(name))
                        .contextMenu {
                            Button("Edit Connection…") { editing = ConnectionEdit(original: name) }
                            Divider()
                            Button("Delete…", role: .destructive) { pendingDeletion = name }
                        }
                    if store.expandedConnections.contains(name) {
                        ForEach(store.tablesByConnection[name] ?? [], id: \.self) { table in
                            Label(table.name, systemImage: "tablecells")
                                .padding(.leading, 18)
                                .tag(DatabaseSelection.table(
                                    DatabaseTable(connection: name, schema: table.schema, name: table.name)
                                ))
                        }
                    }
                }
            }
            .listStyle(.sidebar)
            .environment(\.defaultMinListRowHeight, 26)
            .overlay {
                if store.connections.isEmpty {
                    EmptyPane(
                        "No connections",
                        systemImage: "cylinder.split.1x2",
                        description: Text("Add a PostgreSQL connection with +. Each environment gets its own URL, and queries are read-only unless you allow writes.")
                    )
                }
            }
        }
        .sheet(item: $editing) { edit in
            ConnectionSheet(store: store, original: edit.original)
        }
        .confirmationDialog(
            "Delete connection \"\(pendingDeletion ?? "")\"?",
            isPresented: Binding(get: { pendingDeletion != nil }, set: { if !$0 { pendingDeletion = nil } }),
            titleVisibility: .visible
        ) {
            Button("Delete connection", role: .destructive) {
                guard let name = pendingDeletion else { return }
                Task { await store.deleteConnection(name) }
            }
        } message: {
            Text("Its file .kestrl/connections/\(pendingDeletion ?? "").toml is removed. The database itself and Keychain secrets are not touched.")
        }
        .onChange(of: store.selectedDatabaseItem) { old, _ in
            Task { await store.databaseSelectionChanged(from: old) }
        }
    }

    private func connectionRow(_ name: String) -> some View {
        let expanded = store.expandedConnections.contains(name)
        return HStack(spacing: 6) {
            Button {
                store.setConnectionExpanded(name, expanded: !expanded)
            } label: {
                Image(systemName: "chevron.right")
                    .font(.system(size: 9, weight: .semibold))
                    .rotationEffect(.degrees(expanded ? 90 : 0))
                    .frame(width: 12, height: 16)
            }
            .buttonStyle(.plain)
            .foregroundStyle(.secondary)
            .accessibilityLabel(expanded ? "Collapse \(name)" : "Show tables of \(name)")
            Label(name, systemImage: "cylinder")
                .lineLimit(1)
        }
    }
}
