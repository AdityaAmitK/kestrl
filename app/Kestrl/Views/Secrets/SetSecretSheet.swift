import SwiftUI

/// Enter a secret's value. It goes straight to the secrets file and is never kept in the store.
struct SetSecretSheet: View {
    let store: WorkspaceStore
    let name: String

    @Environment(\.dismiss) private var dismiss
    @State private var value = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Set secret")
                .font(.headline)
            Text(name)
                .font(.body.monospaced())
                .foregroundStyle(.secondary)
            SecureField("Value", text: $value)
                .textFieldStyle(.roundedBorder)
                .onSubmit(save)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
                Button("Save", action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(value.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 380)
    }

    private func save() {
        guard !value.isEmpty else { return }
        let secretValue = value
        Task {
            if await store.saveSecret(name: name, value: secretValue) {
                dismiss()
            }
        }
    }
}
