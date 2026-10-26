import SwiftUI

/// Create or edit a database connection: its name, kind, a URL per environment, and where
/// writes are allowed (read-only everywhere else). URLs can use {{secret:name}} for credentials.
struct ConnectionSheet: View {
    let store: WorkspaceStore
    /// The connection being edited; nil creates a new one
    let original: String?

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var kind = "postgres"
    @State private var rows: [Row] = []
    @State private var isLoading = true
    @State private var isSaving = false

    private struct Row: Identifiable, Equatable {
        let id = UUID()
        var environment: String
        var url: String
        var allowsWrites: Bool
    }

    private var environments: [String] { store.workspace?.envs ?? [] }
    private var trimmedName: String { name.trimmingCharacters(in: .whitespaces) }

    private var problem: String? {
        if trimmedName.isEmpty { return "Enter a name." }
        if let problem = validateName(kind: "environment", name: trimmedName) { return problem }
        if trimmedName != original, store.connections.contains(trimmedName) {
            return "A connection named \"\(trimmedName)\" already exists."
        }
        if rows.isEmpty { return "Add a URL for at least one environment." }
        if rows.contains(where: { $0.url.trimmingCharacters(in: .whitespaces).isEmpty }) { return "Every environment needs a URL." }
        if Set(rows.map(\.environment)).count != rows.count { return "Each environment can appear only once." }
        return nil
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(original == nil ? "New connection" : "Edit \"\(original ?? "")\"").font(.headline)
                Text("Queries are read-only unless an environment allows writes. Keep credentials out of the file with {{secret:name}}.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Form {
                TextField("Name", text: $name, prompt: Text("main"))
                    .accessibilityLabel("Connection name")
                Picker("Kind", selection: $kind) {
                    ForEach(connectionKinds(), id: \.self) { kind in
                        Text(kind == "postgres" ? "PostgreSQL" : kind).tag(kind)
                    }
                }
            }
            .formStyle(.columns)

            VStack(alignment: .leading, spacing: 8) {
                Text("Environments").font(.subheadline.weight(.semibold))
                Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 8) {
                    GridRow {
                        Text("Environment").frame(width: 130, alignment: .leading)
                        Text("URL").frame(minWidth: 280, maxWidth: .infinity, alignment: .leading)
                        Text("Writes")
                        Color.clear.frame(width: 18, height: 1)
                    }
                    .font(.caption.weight(.medium))
                    .foregroundStyle(.secondary)
                    ForEach($rows) { $row in
                        GridRow(alignment: .center) {
                            Picker("Environment", selection: $row.environment) {
                                ForEach(environments, id: \.self) { Text($0).tag($0) }
                            }
                            .labelsHidden()
                            .frame(width: 130)
                            VariableTextView(
                                text: $row.url,
                                variables: store.editorVariables,
                                label: "URL for \(row.environment)",
                                environmentName: row.environment,
                                singleLine: true
                            )
                            .frame(minWidth: 280, maxWidth: .infinity, minHeight: 24, maxHeight: 24)
                            Toggle("Allow writes in \(row.environment)", isOn: $row.allowsWrites)
                                .labelsHidden()
                                .toggleStyle(.switch)
                                .controlSize(.mini)
                                .help(row.allowsWrites ? "INSERT, UPDATE and DELETE are allowed" : "Read-only: Postgres rejects writes")
                            Button {
                                rows.removeAll { $0.id == row.id }
                            } label: {
                                Image(systemName: "minus.circle")
                            }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Remove \(row.environment)")
                        }
                    }
                }
                Button {
                    let unused = environments.first { env in !rows.contains { $0.environment == env } } ?? environments.first ?? "local"
                    rows.append(Row(environment: unused, url: "postgres://{{secret:db_user}}:{{secret:db_password}}@localhost:5432/app", allowsWrites: unused == "local"))
                } label: {
                    Label("Add environment", systemImage: "plus")
                }
                .buttonStyle(.borderless)
                .disabled(rows.count >= environments.count)
            }

            Group {
                if let problem, !isLoading {
                    Label(problem, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                } else {
                    Text("Saved as .kestrl/connections/\(trimmedName.isEmpty ? "name" : trimmedName).toml")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(original == nil ? "Create" : "Save", action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(problem != nil || isSaving || isLoading)
            }
        }
        .padding(20)
        .frame(width: 640)
        .task { await load() }
    }

    private func load() async {
        defer { isLoading = false }
        guard let original, let root = store.workspace?.root else {
            name = ""
            let first = store.workspace?.defaultEnv ?? environments.first ?? "local"
            rows = [Row(environment: first, url: "postgres://{{secret:db_user}}:{{secret:db_password}}@localhost:5432/app", allowsWrites: true)]
            return
        }
        name = original
        do {
            let record = try await KestrlBridge.run { try readConnection(workspaceRoot: root, name: original) }
            kind = record.kind
            rows = record.urls.map { Row(environment: $0.key, url: $0.value, allowsWrites: record.allowWrites.contains($0.key)) }
        } catch {
            store.report(error)
            dismiss()
        }
    }

    private func save() {
        guard problem == nil, let root = store.workspace?.root else { return }
        isSaving = true
        let record = ConnectionRecord(
            kind: kind,
            urls: rows.map { KeyValue(key: $0.environment, value: $0.url.trimmingCharacters(in: .whitespaces)) },
            allowWrites: rows.filter(\.allowsWrites).map(\.environment)
        )
        let target = trimmedName
        let previous = original
        Task {
            do {
                try await KestrlBridge.run {
                    try saveConnection(workspaceRoot: root, name: target, previous: previous, connection: record)
                }
                await store.connectionsChanged(selecting: target)
                dismiss()
            } catch {
                store.report(error)
            }
            isSaving = false
        }
    }
}
