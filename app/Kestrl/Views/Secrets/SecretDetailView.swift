import SwiftUI

/// The selected secret: its status, where it is used, and set / replace / remove. Values are never shown.
struct SecretDetailView: View {
    @Bindable var store: WorkspaceStore

    @State private var confirmingRemoval = false

    var body: some View {
        if let name = store.selectedSecret, let secret = store.secrets.first(where: { $0.name == name }) {
            VStack(spacing: 0) {
                PaneHeader(secret.name, subtitle: secret.isSet ? "Stored in the Keychain" : "Not set") {
                    Button(secret.isSet ? "Replace Value…" : "Set Value…") {
                        store.isSettingSecret = true
                    }
                    .buttonStyle(.borderedProminent)
                    Button("Remove…", role: .destructive) {
                        confirmingRemoval = true
                    }
                    .disabled(!secret.isSet)
                    .help(secret.isSet ? "Delete the value from the Keychain" : "Nothing is stored yet")
                }
                Divider()
                Form {
                    LabeledContent("Status") {
                        Label(
                            secret.isSet ? "Stored in the macOS Keychain" : "Not set — requests that use it fail until you set it",
                            systemImage: secret.isSet ? "checkmark.circle.fill" : "exclamationmark.triangle.fill"
                        )
                        .foregroundStyle(secret.isSet ? Color.green : Color.orange)
                    }
                    LabeledContent("Reference") {
                        Text("{{secret:\(secret.name)}}")
                            .font(.body.monospaced())
                            .textSelection(.enabled)
                    }
                    LabeledContent("Used by") {
                        Text(secret.isReferenced ? "Requests or environments in this workspace" : "Nothing in this workspace yet")
                            .foregroundStyle(.secondary)
                    }
                    LabeledContent("Value") {
                        Text("Never shown. Kestrl reads it only when it sends a request.")
                            .foregroundStyle(.secondary)
                    }
                }
                .formStyle(.grouped)
                .scrollContentBackground(.hidden)
            }
            .confirmationDialog("Remove “\(secret.name)”?", isPresented: $confirmingRemoval, titleVisibility: .visible) {
                Button("Remove from Keychain", role: .destructive) {
                    Task { await store.removeSecret(secret.name) }
                }
            } message: {
                Text(secret.isReferenced
                     ? "Requests that use {{secret:\(secret.name)}} fail until it is set again."
                     : "The value is deleted from the Keychain.")
            }
            .sheet(isPresented: $store.isSettingSecret) {
                SetSecretSheet(store: store, name: secret.name)
            }
        } else {
            EmptyPane(
                "Select a secret",
                systemImage: "key",
                description: Text("Its status and actions appear here.")
            )
        }
    }
}
