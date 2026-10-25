import SwiftUI

/// Secret names the workspace refers to, and whether the Keychain holds each one.
struct SecretsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedSecret) {
            ForEach(store.secrets) { secret in
                HStack {
                    Label(secret.name, systemImage: "key")
                        .lineLimit(1)
                    Spacer()
                    Image(systemName: secret.isSet ? "checkmark.circle" : "exclamationmark.circle")
                        .foregroundStyle(.secondary)
                        .help(secret.isSet ? "Set" : "Missing")
                }
                .tag(secret.name)
            }
        }
        .listStyle(.plain)
        .overlay {
            if store.secrets.isEmpty {
                EmptyPane(
                    "No secrets",
                    systemImage: "key",
                    description: Text("Use {{secret:name}} in a request or environment to add one.")
                )
            }
        }
    }
}
