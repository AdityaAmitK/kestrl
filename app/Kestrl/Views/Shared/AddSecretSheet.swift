import SwiftUI

/// Store a new secret. `onAdded` runs after the value is stored, e.g. to add
/// `name = "{{secret:name}}"` to an environment.
struct AddSecretSheet: View {
    let store: WorkspaceStore
    var explanation = "The value goes to Kestrl's secrets file. Workspace files only store a {{secret:name}} reference, so they are safe to commit."
    var onAdded: (String) -> Void = { _ in }

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var value = ""
    @State private var isSaving = false
    @FocusState private var isNameFocused: Bool

    private var trimmedName: String { name.trimmingCharacters(in: .whitespaces) }

    private var problem: String? {
        if trimmedName.isEmpty { return nil }
        return validateName(kind: "variable", name: trimmedName)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Add secret").font(.headline)
                Text(explanation)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Form {
                TextField("Name", text: $name, prompt: Text("api_token"))
                    .focused($isNameFocused)
                    .accessibilityLabel("Secret name")
                SecureField("Value", text: $value, prompt: Text("Never shown again"))
                    .accessibilityLabel("Secret value")
                    .onSubmit(save)
            }
            .formStyle(.columns)

            Group {
                if let problem {
                    Label(problem, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                } else {
                    Text(trimmedName.isEmpty ? "Use it as {{secret:name}}." : "Use it as {{secret:\(trimmedName)}}.")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.caption)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button("Add", action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(trimmedName.isEmpty || value.isEmpty || problem != nil || isSaving)
            }
        }
        .padding(20)
        .frame(width: 420)
        .onAppear { isNameFocused = true }
    }

    private func save() {
        guard !trimmedName.isEmpty, !value.isEmpty, problem == nil, !isSaving else { return }
        isSaving = true
        let secretName = trimmedName
        let secretValue = value
        Task {
            let isStored = await store.saveSecret(name: secretName, value: secretValue)
            isSaving = false
            guard isStored else { return }
            onAdded(secretName)
            dismiss()
        }
    }
}
