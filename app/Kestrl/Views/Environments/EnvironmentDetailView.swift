import SwiftUI

/// Edit the variables of the selected environment. Changes are drafts until Save (⌘S).
struct EnvironmentDetailView: View {
    @Bindable var store: WorkspaceStore
    @State private var isAddingSecret = false

    private var editor: EnvironmentEditor { store.environmentEditor }

    var body: some View {
        @Bindable var editor = editor
        if let name = editor.loadedEnvironment, name == store.inspectedEnvironment {
            VStack(spacing: 0) {
                header(name)
                Divider()
                if editor.rows.isEmpty {
                    emptyState
                } else {
                    variableTable(editor)
                }
                Divider()
                footer
            }
            .sheet(isPresented: $isAddingSecret) {
                NewSecretVariableSheet(store: store)
            }
            .sheet(item: Binding(
                get: { editor.secretToSet.map(SecretName.init) },
                set: { editor.secretToSet = $0?.name }
            )) { secret in
                SetSecretSheet(store: store, name: secret.name)
            }
        } else {
            EmptyPane(
                "Select an environment",
                systemImage: "globe",
                description: Text("Its variables appear here, ready to edit.")
            )
        }
    }

    // MARK: Header and footer

    private func header(_ name: String) -> some View {
        PaneHeader(name, subtitle: subtitle(for: name)) {
            if editor.hasChanges {
                Button("Revert") { editor.revert() }
                    .help("Discard unsaved changes")
            }
            if editor.hasChanges {
                saveButton(name).buttonStyle(.borderedProminent)
            } else {
                saveButton(name).buttonStyle(.bordered)
            }
        }
    }

    private func saveButton(_ name: String) -> some View {
        Button {
            Task { await editor.save() }
        } label: {
            Text(editor.isSaving ? "Saving…" : "Save")
        }
        .keyboardShortcut("s", modifiers: .command)
        .disabled(!editor.hasChanges || editor.validationMessage != nil || editor.isSaving)
        .help("Save to .kestrl/envs/\(name).toml (⌘S)")
    }

    private func subtitle(for name: String) -> String {
        var parts: [String] = []
        if store.workspace?.defaultEnv == name { parts.append("Default") }
        parts.append(editor.rows.count == 1 ? "1 variable" : "\(editor.rows.count) variables")
        if editor.hasChanges { parts.append("Edited") }
        return parts.joined(separator: " · ")
    }

    private var footer: some View {
        HStack(spacing: 12) {
            Button {
                editor.addRow()
            } label: {
                Label("Add variable", systemImage: "plus")
            }
            Button {
                isAddingSecret = true
            } label: {
                Label("Add secret…", systemImage: "key")
            }
            .help("Add a variable whose value lives in the macOS Keychain")
            Spacer()
            if let message = editor.validationMessage, !editor.rows.isEmpty {
                Label(message, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(.orange)
                    .lineLimit(1)
            } else {
                Text("Use {{name}} in requests. Secrets are stored in the Keychain, never in the file.")
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        }
        .font(.callout)
        .buttonStyle(.borderless)
        .padding(.horizontal, 12)
        .frame(height: 36)
    }

    private var emptyState: some View {
        EmptyPane(
            "No variables yet",
            systemImage: "list.bullet.rectangle",
            description: Text("Add a variable such as api_url, then use it in a request as {{api_url}}.")
        )
    }

    // MARK: Table

    private func variableTable(_ editor: EnvironmentEditor) -> some View {
        @Bindable var editor = editor
        return ScrollView {
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 8) {
                GridRow {
                    Text("Name")
                    Text("Value")
                    Text("Resolves to")
                    Color.clear.frame(width: 20, height: 1)
                }
                .font(.caption.weight(.medium))
                .foregroundStyle(.secondary)
                Divider().gridCellUnsizedAxes(.horizontal)
                ForEach($editor.rows) { $row in
                    EnvironmentVariableRow(
                        row: $row,
                        problem: editor.problem(for: row),
                        resolved: resolvedValue(for: row),
                        variables: editor.catalog,
                        environmentName: editor.loadedEnvironment,
                        onRemove: { editor.removeRows([row.id]) },
                        onSetSecret: { editor.secretToSet = secretName(in: row.value) }
                    )
                }
            }
            .padding(12)
        }
    }

    /// The saved value with references expanded and secrets masked. Unsaved rows show no preview yet.
    /// Uses the variable catalog, which never fails on a missing secret.
    private func resolvedValue(for row: EnvironmentRow) -> String? {
        guard editor.savedRows.contains(row) else { return nil }
        return editor.catalog.first { $0.name == row.name }?.value
    }

    private func secretName(in value: String) -> String? {
        guard let start = value.range(of: "{{secret:"),
              let end = value.range(of: "}}", range: start.upperBound..<value.endIndex) else { return nil }
        return String(value[start.upperBound..<end.lowerBound])
    }
}

/// Wraps a secret name so it can drive `.sheet(item:)`.
private struct SecretName: Identifiable {
    let name: String
    var id: String { name }
}
