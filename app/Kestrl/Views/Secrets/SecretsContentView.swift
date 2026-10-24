import SwiftUI

/// Secret names the workspace refers to, and whether the Keychain holds each one.
struct SecretsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        Table(store.secrets, selection: $store.selectedSecret) {
            TableColumn("Name") { secret in
                Text(secret.name)
                    .font(.body.monospaced())
            }
            .width(min: 140, ideal: 200)

            TableColumn("Status") { secret in
                Label(
                    secret.isSet ? "Set" : "Missing",
                    systemImage: secret.isSet ? "checkmark.circle" : "exclamationmark.circle"
                )
                .foregroundStyle(.secondary)
            }
            .width(min: 90, ideal: 110)
        }
        .overlay {
            if store.secrets.isEmpty {
                ContentUnavailableView(
                    "No secrets",
                    systemImage: "key",
                    description: Text("Use {{secret:name}} in a request or environment to add one.")
                )
            }
        }
        .navigationTitle("Secrets")
    }
}
