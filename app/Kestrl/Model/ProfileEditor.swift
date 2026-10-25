import Foundation
import Observation

/// A profile being edited. Headers and query use the same `name: value` / `name=value` lines as
/// request drafts, so the shared key/value table edits them.
struct ProfileDraft: Equatable {
    var headers = ""
    var query = ""
    /// Apply automatically when the conditions match, instead of only when requested
    var isAutomatic = false
    var environments: Set<String> = []
    var urlPattern = ""

    init() {}

    init(_ record: ProfileRecord) {
        headers = record.headers.map { "\($0.key): \($0.value)" }.joined(separator: "\n")
        query = record.query.map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
        environments = Set(record.whenEnvs)
        urlPattern = record.whenUrl ?? ""
        isAutomatic = !record.whenEnvs.isEmpty || record.whenUrl != nil
    }

    func record() -> ProfileRecord {
        ProfileRecord(
            headers: Self.pairs(headers, separator: ":"),
            query: Self.pairs(query, separator: "="),
            whenEnvs: isAutomatic ? environments.sorted() : [],
            whenUrl: isAutomatic && !urlPattern.trimmingCharacters(in: .whitespaces).isEmpty
                ? urlPattern.trimmingCharacters(in: .whitespaces) : nil
        )
    }

    private static func pairs(_ lines: String, separator: Character) -> [KeyValue] {
        lines.split(separator: "\n").compactMap { line in
            guard let index = line.firstIndex(of: separator) else { return nil }
            let key = line[..<index].trimmingCharacters(in: .whitespaces)
            guard !key.isEmpty else { return nil }
            return KeyValue(key: key, value: line[line.index(after: index)...].trimmingCharacters(in: .whitespaces))
        }
    }
}

/// Draft edits for the inspected profile, and the create / duplicate / rename / delete actions.
@Observable
@MainActor
final class ProfileEditor {
    private unowned let store: WorkspaceStore

    var draft = ProfileDraft()
    private(set) var saved = ProfileDraft()
    private(set) var loadedProfile: String?
    private(set) var isSaving = false
    /// The profile shown in the detail pane
    var inspected: String?
    var nameIntent: NameIntent?
    var pendingDeletion: String?
    var pendingSelection: String?

    init(store: WorkspaceStore) {
        self.store = store
    }

    var hasChanges: Bool { draft != saved }

    /// The first problem that blocks saving.
    var validationMessage: String? {
        if draft.isAutomatic && draft.environments.isEmpty && draft.urlPattern.trimmingCharacters(in: .whitespaces).isEmpty {
            return "Choose at least one environment or a URL pattern, or turn off automatic use."
        }
        return nil
    }

    func load(_ name: String?) async {
        guard let root = store.workspace?.root, let name else {
            draft = ProfileDraft()
            saved = draft
            loadedProfile = nil
            return
        }
        do {
            let record = try await KestrlBridge.run { try readProfile(workspaceRoot: root, name: name) }
            draft = ProfileDraft(record)
            saved = draft
            loadedProfile = name
        } catch {
            store.report(error)
        }
    }

    @discardableResult
    func save() async -> Bool {
        guard let root = store.workspace?.root, let name = loadedProfile, validationMessage == nil else { return false }
        isSaving = true
        defer { isSaving = false }
        let record = draft.record()
        do {
            try await KestrlBridge.run { try saveProfile(workspaceRoot: root, name: name, profile: record) }
            // Re-read so the draft matches exactly what the file now holds.
            let reloaded = try await KestrlBridge.run { try readProfile(workspaceRoot: root, name: name) }
            draft = ProfileDraft(reloaded)
            saved = draft
            return true
        } catch {
            store.report(error)
            return false
        }
    }

    func revert() {
        draft = saved
    }

    func commitName(_ name: String, intent: NameIntent, copyFrom: String?) async -> Bool {
        guard let root = store.workspace?.root else { return false }
        do {
            switch intent {
            case .create:
                try await KestrlBridge.run { try createProfile(workspaceRoot: root, name: name, copyFrom: copyFrom) }
            case .duplicate(let source):
                try await KestrlBridge.run { try createProfile(workspaceRoot: root, name: name, copyFrom: source) }
            case .rename(let current):
                guard current != name else { return true }
                try await KestrlBridge.run { try renameProfile(workspaceRoot: root, from: current, to: name) }
            }
            await store.reload()
            inspected = name
            await load(name)
            return true
        } catch {
            store.report(error)
            return false
        }
    }

    func delete(_ name: String) async {
        guard let root = store.workspace?.root else { return }
        do {
            try await KestrlBridge.run { try deleteProfile(workspaceRoot: root, name: name) }
            await store.reload()
            if inspected == name {
                inspected = store.workspace?.profiles.first
                await load(inspected)
            }
        } catch {
            store.report(error)
        }
    }
}
