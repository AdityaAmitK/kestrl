import SwiftUI

/// Name a new, duplicated or renamed environment. New environments can start empty or as a copy.
struct EnvironmentNameSheet: View {
    let store: WorkspaceStore
    let intent: EnvironmentNameIntent

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var copyFrom: String?
    @State private var isWorking = false
    @FocusState private var isNameFocused: Bool

    private var editor: EnvironmentEditor { store.environmentEditor }
    private var problem: String? { editor.nameProblem(name, intent: intent) }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.headline)
                Text(explanation)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            Form {
                TextField("Name", text: $name, prompt: Text("staging"))
                    .focused($isNameFocused)
                    .onSubmit(commit)
                    .accessibilityLabel("Environment name")
                if intent == .create {
                    Picker("Start from", selection: $copyFrom) {
                        Text("Empty").tag(String?.none)
                        if !(store.workspace?.envs.isEmpty ?? true) {
                            Divider()
                            ForEach(store.workspace?.envs ?? [], id: \.self) { env in
                                Text("Copy of \(env)").tag(Optional(env))
                            }
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
                    Text("Saved as .kestrl/envs/\(trimmedName.isEmpty ? "name" : trimmedName).toml")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(actionTitle, action: commit)
                    .keyboardShortcut(.defaultAction)
                    .disabled(problem != nil || isWorking)
            }
        }
        .padding(20)
        .frame(width: 420)
        .onAppear(perform: prefill)
    }

    private var trimmedName: String { name.trimmingCharacters(in: .whitespaces) }

    private var title: String {
        switch intent {
        case .create: return "New environment"
        case .duplicate(let source): return "Duplicate \"\(source)\""
        case .rename(let current): return "Rename \"\(current)\""
        }
    }

    private var explanation: String {
        switch intent {
        case .create: return "An environment is a set of variables, such as api_url, that requests resolve when you send them."
        case .duplicate: return "The copy keeps every variable and comment. Secret values stay in the Keychain and are shared."
        case .rename: return "Requests keep working. If this is the default environment, the default follows the new name."
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
        let existing = Set(store.workspace?.envs ?? [])
        guard existing.contains(base) else { return base }
        var number = 2
        while existing.contains("\(base)-\(number)") { number += 1 }
        return "\(base)-\(number)"
    }

    private func commit() {
        guard problem == nil, !isWorking else { return }
        isWorking = true
        let source = copyFrom
        Task {
            let isDone = await editor.commitName(name, intent: intent, copyFrom: source)
            isWorking = false
            if isDone { dismiss() }
        }
    }
}
