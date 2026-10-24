import SwiftUI

/// The selected secret: its status, and the actions to set or remove it. Values are never shown.
struct SecretDetailView: View {
    @Bindable var store: WorkspaceStore

    @State private var confirmingRemoval = false

    var body: some View {
        if let name = store.selectedSecret, let secret = store.secrets.first(where: { $0.name == name }) {
            VStack(alignment: .leading, spacing: 16) {
                Text(secret.name)
                    .font(.title2.weight(.semibold).monospaced())
                Label(secret.isSet ? "Stored in the Keychain" : "Not set", systemImage: secret.isSet ? "checkmark.circle" : "exclamationmark.circle")
                    .foregroundStyle(.secondary)
                Text("Requests and environments use it as {{secret:\(secret.name)}}. Its value is never shown.")
                    .foregroundStyle(.secondary)

                HStack {
                    Button("Set…") {
                        store.isSettingSecret = true
                    }
                    .buttonStyle(.borderedProminent)

                    Button("Remove…", role: .destructive) {
                        confirmingRemoval = true
                    }
                    .disabled(!secret.isSet)
                }
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .confirmationDialog("Remove “\(secret.name)”?", isPresented: $confirmingRemoval, titleVisibility: .visible) {
                Button("Remove", role: .destructive) {
                    Task { await store.removeSecret(secret.name) }
                }
            } message: {
                Text("Requests that use this secret fail until it is set again.")
            }
            .sheet(isPresented: $store.isSettingSecret) {
                SetSecretSheet(store: store, name: secret.name)
            }
        } else {
            ContentUnavailableView(
                "Select a secret",
                systemImage: "key",
                description: Text("Its status and actions appear here.")
            )
        }
    }
}
