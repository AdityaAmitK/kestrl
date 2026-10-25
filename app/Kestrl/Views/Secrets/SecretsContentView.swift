import SwiftUI

/// Secrets the workspace refers to or the Keychain holds for Kestrl. Values are never shown.
struct SecretsContentView: View {
    @Bindable var store: WorkspaceStore
    @State private var isAdding = false

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader("Secrets", subtitle: store.secrets.isEmpty ? nil : "\(store.secrets.count)") {
                Button {
                    isAdding = true
                } label: {
                    Image(systemName: "plus")
                }
                .buttonStyle(.borderless)
                .help("Add a secret to the Keychain")
                .accessibilityLabel("Add secret")
            }
            Divider()
            List(selection: $store.selectedSecret) {
                ForEach(store.secrets) { secret in
                    row(secret).tag(secret.name)
                }
            }
            .listStyle(.sidebar)
            .environment(\.defaultMinListRowHeight, 28)
            .overlay {
                if store.secrets.isEmpty {
                    EmptyPane(
                        "No secrets",
                        systemImage: "key",
                        description: Text("Add one with +, then use it as {{secret:name}} in a request or environment. Values stay in the macOS Keychain.")
                    )
                }
            }
        }
        .sheet(isPresented: $isAdding) {
            AddSecretSheet(store: store) { name in
                store.selectedSecret = name
            }
        }
        .onChange(of: store.secrets.map(\.name)) { _, names in
            if store.selectedSecret.map({ !names.contains($0) }) ?? true {
                store.selectedSecret = names.first
            }
        }
        .onAppear {
            if store.selectedSecret == nil { store.selectedSecret = store.secrets.first?.name }
        }
    }

    private func row(_ secret: SecretStatus) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "key")
                .foregroundStyle(.secondary)
            Text(secret.name)
                .font(.body.monospaced())
                .lineLimit(1)
            Spacer(minLength: 4)
            if !secret.isReferenced {
                Text("Unused")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(.quaternary, in: Capsule())
                    .help("No request or environment uses this secret")
            }
            Image(systemName: secret.isSet ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                .foregroundStyle(secret.isSet ? Color.green : Color.orange)
                .help(secret.isSet ? "Stored in the Keychain" : "Not set")
                .accessibilityLabel(secret.isSet ? "Set" : "Not set")
        }
    }
}
