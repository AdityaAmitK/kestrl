import Foundation
import Observation

/// One editable variable row. The id keeps SwiftUI focus stable while a name is being typed.
struct EnvironmentRow: Identifiable, Equatable {
    let id = UUID()
    var name: String
    var value: String

    static func == (lhs: Self, rhs: Self) -> Bool { lhs.name == rhs.name && lhs.value == rhs.value }

    var isSecretReference: Bool { value.contains("{{secret:") }
}

/// Draft variables of the inspected environment, and the create / duplicate / rename / delete actions.
/// Files are only written on Save, so a half-typed row never reaches disk.
@Observable
@MainActor
final class EnvironmentEditor {
    private unowned let store: WorkspaceStore

    /// The rows being edited, in file order.
    var rows: [EnvironmentRow] = []
    /// The rows as last loaded or saved.
    private(set) var savedRows: [EnvironmentRow] = []
    /// Completions and hover values for the loaded environment (secret values never included).
    private(set) var catalog: [VariableInfo] = []
    /// The environment whose rows are loaded.
    private(set) var loadedEnvironment: String?
    private(set) var isSaving = false

    /// The open name sheet, if any.
    var nameIntent: NameIntent?
    /// The environment waiting for delete confirmation.
    var pendingDeletion: String?
    /// A selection the user made while edits were unsaved.
    var pendingSelection: String?
    /// The secret whose value sheet is open after "Add secret".
    var secretToSet: String?

    init(store: WorkspaceStore) {
        self.store = store
    }

    var hasChanges: Bool { rows != savedRows }

    /// The first problem that blocks saving, shown inline under the table.
    var validationMessage: String? {
        var seen = Set<String>()
        for row in rows {
            let name = row.name.trimmingCharacters(in: .whitespaces)
            if name.isEmpty { return "Every variable needs a name." }
            if let problem = validateName(kind: "variable", name: name) { return problem }
            if !seen.insert(name).inserted { return "\"\(name)\" is defined twice." }
        }
        return nil
    }

    func problem(for row: EnvironmentRow) -> String? {
        let name = row.name.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty else { return nil }
        if let problem = validateName(kind: "variable", name: name) { return problem }
        return rows.filter({ $0.name.trimmingCharacters(in: .whitespaces) == name }).count > 1 ? "Defined twice" : nil
    }

    // MARK: Loading and saving

    func load(_ environment: String?) async {
        guard let root = store.workspace?.root, let environment else {
            rows = []
            savedRows = []
            catalog = []
            loadedEnvironment = nil
            return
        }
        do {
            let pairs = try await KestrlBridge.run { try environmentVariables(workspaceRoot: root, env: environment) }
            let found = try await KestrlBridge.run { try variableCatalog(workspaceRoot: root, env: environment) }
            let loaded = pairs.map { EnvironmentRow(name: $0.key, value: $0.value) }
            catalog = found
            rows = loaded
            savedRows = loaded
            loadedEnvironment = environment
        } catch {
            store.report(error)
        }
    }

    /// Returns true when the file was written, so a pending selection can continue.
    @discardableResult
    func save() async -> Bool {
        guard let root = store.workspace?.root, let environment = loadedEnvironment, validationMessage == nil else {
            return false
        }
        isSaving = true
        defer { isSaving = false }
        let trimmed = rows.map { EnvironmentRow(name: $0.name.trimmingCharacters(in: .whitespaces), value: $0.value) }
        let pairs = trimmed.map { KeyValue(key: $0.name, value: $0.value) }
        do {
            try await KestrlBridge.run {
                try saveEnvironmentVariables(workspaceRoot: root, env: environment, variables: pairs)
            }
            rows = trimmed
            savedRows = trimmed
            catalog = (try? await KestrlBridge.run { try variableCatalog(workspaceRoot: root, env: environment) }) ?? catalog
            await store.environmentFilesChanged()
            return true
        } catch {
            store.report(error)
            return false
        }
    }

    func revert() {
        rows = savedRows
    }

    // MARK: Rows

    func addRow() {
        rows.append(EnvironmentRow(name: "", value: ""))
    }

    func removeRows(_ ids: Set<EnvironmentRow.ID>) {
        rows.removeAll { ids.contains($0.id) }
    }

    /// Adds `name = "{{secret:name}}"` and asks for the Keychain value.
    func addSecret(named name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        if let index = rows.firstIndex(where: { $0.name == trimmed }) {
            rows[index].value = "{{secret:\(trimmed)}}"
        } else {
            rows.append(EnvironmentRow(name: trimmed, value: "{{secret:\(trimmed)}}"))
        }
        secretToSet = trimmed
    }

    // MARK: Environments

    /// Creates, duplicates or renames, then selects the result. Returns true on success.
    func commitName(_ name: String, intent: NameIntent, copyFrom: String?) async -> Bool {
        guard let root = store.workspace?.root else { return false }
        let target = name.trimmingCharacters(in: .whitespaces)
        do {
            switch intent {
            case .create:
                try await KestrlBridge.run { try createEnvironment(workspaceRoot: root, name: target, copyFrom: copyFrom) }
            case .duplicate(let source):
                try await KestrlBridge.run { try createEnvironment(workspaceRoot: root, name: target, copyFrom: source) }
            case .rename(let current):
                guard current != target else { return true }
                try await KestrlBridge.run { try renameEnvironment(workspaceRoot: root, from: current, to: target) }
                if store.selectedEnvironment == current { store.selectedEnvironment = target }
            }
            await store.environmentFilesChanged()
            store.inspectedEnvironment = target
            await load(target)
            return true
        } catch {
            store.report(error)
            return false
        }
    }

    func makeDefault(_ name: String) async {
        guard let root = store.workspace?.root else { return }
        do {
            try await KestrlBridge.run { try setDefaultEnvironment(workspaceRoot: root, name: name) }
            await store.environmentFilesChanged()
        } catch {
            store.report(error)
        }
    }

    func delete(_ name: String) async {
        guard let root = store.workspace?.root else { return }
        do {
            try await KestrlBridge.run { try deleteEnvironment(workspaceRoot: root, name: name) }
            await store.environmentFilesChanged()
            if store.inspectedEnvironment == name {
                store.inspectedEnvironment = store.workspace?.defaultEnv ?? store.workspace?.envs.first
                await load(store.inspectedEnvironment)
            }
        } catch {
            store.report(error)
        }
    }
}
