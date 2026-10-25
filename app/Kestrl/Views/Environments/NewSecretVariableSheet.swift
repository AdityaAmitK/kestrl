import SwiftUI

/// Add a variable whose value lives in the Keychain: the file gets `name = "{{secret:name}}"`.
struct NewSecretVariableSheet: View {
    let store: WorkspaceStore

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
                Text("The value goes to the macOS Keychain. The environment file only stores a reference, so it is safe to commit.")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Form {
                TextField("Name", text: $name, prompt: Text("api_token"))
                    .focused($isNameFocused)
                    .accessibilityLabel("Secret name")
                SecureField("Value", text: $value, prompt: Text("Stored in the Keychain"))
                    .accessibilityLabel("Secret value")
                    .onSubmit(save)
            }
            .formStyle(.columns)

            Group {
                if let problem {
                    Label(problem, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                } else {
                    Text(trimmedName.isEmpty ? "Requests use it as {{name}}." : "Requests use it as {{\(trimmedName)}}.")
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
            store.environmentEditor.addSecret(named: secretName)
            store.environmentEditor.secretToSet = nil
            dismiss()
        }
    }
}
