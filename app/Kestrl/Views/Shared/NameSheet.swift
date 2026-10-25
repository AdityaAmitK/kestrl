import SwiftUI

/// What a name sheet names: wording, where the file lives, and what already exists.
struct NameSheetKind {
    /// Lowercase noun, e.g. "environment"
    let noun: String
    /// `validateName(kind:)` kind; file-backed kinds share the same rules
    let validationKind: String
    /// Directory under the workspace root, e.g. ".kestrl/envs"
    let directory: String
    let existing: [String]
    let explanation: (NameIntent) -> String
    /// Placeholder for the name field
    var example = "staging"
}

/// Name a new, duplicated or renamed item. New items can start empty or as a copy of another.
struct NameSheet: View {
    let kind: NameSheetKind
    let intent: NameIntent
    /// Creates, duplicates or renames; returns true when the sheet can close.
    let commit: (_ name: String, _ copyFrom: String?) async -> Bool

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var copyFrom: String?
    @State private var isWorking = false
    @FocusState private var isNameFocused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.headline)
                Text(kind.explanation(intent))
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Form {
                TextField("Name", text: $name, prompt: Text(kind.example))
                    .focused($isNameFocused)
                    .onSubmit(save)
                    .accessibilityLabel("\(kind.noun.capitalized) name")
                if intent == .create, !kind.existing.isEmpty {
                    Picker("Start from", selection: $copyFrom) {
                        Text("Empty").tag(String?.none)
                        Divider()
                        ForEach(kind.existing, id: \.self) { item in
                            Text("Copy of \(item)").tag(Optional(item))
                        }
                    }
                }
            }
            .formStyle(.columns)

            Group {
                if let problem, !name.isEmpty {
                    Label(problem, systemImage: "exclamationmark.triangle")
                        .foregroundStyle(.orange)
                } else {
                    Text("Saved as \(kind.directory)/\(trimmedName.isEmpty ? "name" : trimmedName).toml")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(actionTitle, action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(problem != nil || isWorking)
            }
        }
        .padding(20)
        .frame(width: 420)
        .onAppear(perform: prefill)
    }

    private var trimmedName: String { name.trimmingCharacters(in: .whitespaces) }

    private var problem: String? {
        if trimmedName.isEmpty { return "Enter a name." }
        if let problem = validateName(kind: kind.validationKind, name: trimmedName) { return problem }
        if case .rename(let current) = intent, current == trimmedName { return nil }
        if kind.existing.contains(trimmedName) { return "A \(kind.noun) named \"\(trimmedName)\" already exists." }
        return nil
    }

    private var title: String {
        switch intent {
        case .create: return "New \(kind.noun)"
        case .duplicate(let source): return "Duplicate \"\(source)\""
        case .rename(let current): return "Rename \"\(current)\""
        }
    }

    private var actionTitle: String {
        switch intent {
        case .create: return "Create"
        case .duplicate: return "Duplicate"
        case .rename: return "Rename"
        }
    }

    private func prefill() {
        switch intent {
        case .create:
            name = ""
            copyFrom = nil
        case .duplicate(let source):
            name = uniqueName(base: "\(source)-copy")
        case .rename(let current):
            name = current
        }
        isNameFocused = true
    }

    private func uniqueName(base: String) -> String {
        guard kind.existing.contains(base) else { return base }
        var number = 2
        while kind.existing.contains("\(base)-\(number)") { number += 1 }
        return "\(base)-\(number)"
    }

    private func save() {
        guard problem == nil, !isWorking else { return }
        isWorking = true
        let source: String? = switch intent {
        case .duplicate(let source): source
        default: copyFrom
        }
        let target = trimmedName
        Task {
            let isDone = await commit(target, source)
            isWorking = false
            if isDone { dismiss() }
        }
    }
}
