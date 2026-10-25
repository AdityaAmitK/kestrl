import SwiftUI

/// Edit a collection folder's `folder.toml`: a default environment, variables for every
/// environment, and environment-specific overrides. They apply to every request inside.
struct FolderSettingsSheet: View {
    let store: WorkspaceStore
    let folder: String

    @Environment(\.dismiss) private var dismiss
    @State private var defaultEnvironment: String?
    /// "" holds variables for every environment; other keys are environment names.
    @State private var variables: [String: String] = [:]
    @State private var scope = ""
    @State private var isLoading = true
    @State private var isSaving = false

    private var environments: [String] { store.workspace?.envs ?? [] }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Folder settings — \(folder)").font(.headline)
                Text("Applies to every request in this folder and its subfolders. Folder variables win over environment variables; a value passed with --var wins over both.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Form {
                Picker("Default environment", selection: $defaultEnvironment) {
                    Text("Workspace default").tag(String?.none)
                    Divider()
                    ForEach(environments, id: \.self) { Text($0).tag(Optional($0)) }
                }
                .help("Used when no environment is chosen, and selected in the toolbar when you open a request here")
            }
            .formStyle(.columns)

            VStack(alignment: .leading, spacing: 6) {
                Picker("Variables for", selection: $scope) {
                    Text(title(for: "")).tag("")
                    ForEach(environments, id: \.self) { Text(title(for: $0)).tag($0) }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
                Text(scope.isEmpty
                     ? "Used in every environment, unless an environment below overrides the same name."
                     : "Used only in \(scope); these win over the folder's variables for every environment.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                KeyValueLinesEditor(
                    text: Binding(get: { variables[scope] ?? "" }, set: { variables[scope] = $0 }),
                    separator: "=",
                    keyPlaceholder: "variable",
                    valuePlaceholder: "value",
                    variables: store.editorVariables,
                    environmentName: scope.isEmpty ? store.selectedEnvironment : scope
                )
                .id(scope)
                .frame(height: 220)
                .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 6))
            }

            Text("Saved as \(folder)/folder.toml")
                .font(.caption)
                .foregroundStyle(.secondary)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("Save", action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(isLoading || isSaving)
            }
        }
        .padding(20)
        .frame(width: 640)
        .task { await load() }
    }

    private func title(for scope: String) -> String {
        let count = (variables[scope] ?? "").split(separator: "\n").filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }.count
        let name = scope.isEmpty ? "All environments" : scope
        return count > 0 ? "\(name) \(count)" : name
    }

    private func load() async {
        defer { isLoading = false }
        guard let root = store.workspace?.root else { return }
        do {
            let settings = try await KestrlBridge.run { try readFolderSettings(workspaceRoot: root, folder: folder) }
            defaultEnvironment = settings.defaultEnv
            variables[""] = Self.lines(settings.variables)
            for entry in settings.environmentVariables {
                variables[entry.environment] = Self.lines(entry.variables)
            }
        } catch {
            store.report(error)
        }
    }

    private func save() {
        guard let root = store.workspace?.root else { return }
        isSaving = true
        let record = FolderSettingsRecord(
            defaultEnv: defaultEnvironment,
            variables: Self.pairs(variables[""] ?? ""),
            environmentVariables: environments.compactMap { env in
                let pairs = Self.pairs(variables[env] ?? "")
                return pairs.isEmpty ? nil : FolderEnvironmentVariables(environment: env, variables: pairs)
            }
        )
        Task {
            do {
                try await KestrlBridge.run { try saveFolderSettings(workspaceRoot: root, folder: folder, settings: record) }
                await store.loadEditorVariables()
                dismiss()
            } catch {
                store.report(error)
            }
            isSaving = false
        }
    }

    private static func lines(_ pairs: [KeyValue]) -> String {
        pairs.map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
    }

    private static func pairs(_ lines: String) -> [KeyValue] {
        lines.split(separator: "\n").compactMap { line in
            guard let index = line.firstIndex(of: "=") else { return nil }
            let key = line[..<index].trimmingCharacters(in: .whitespaces)
            guard !key.isEmpty else { return nil }
            return KeyValue(key: key, value: line[line.index(after: index)...].trimmingCharacters(in: .whitespaces))
        }
    }
}
