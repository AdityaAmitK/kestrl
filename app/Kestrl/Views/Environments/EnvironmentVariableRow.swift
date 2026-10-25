import SwiftUI

/// One editable variable: name, value with {{…}} highlighting, its resolved preview, and remove.
struct EnvironmentVariableRow: View {
    @Binding var row: EnvironmentRow
    let problem: String?
    let resolved: String?
    let variables: [VariableInfo]
    let environmentName: String?
    let onRemove: () -> Void
    let onSetSecret: () -> Void

    var body: some View {
        GridRow(alignment: .top) {
            VStack(alignment: .leading, spacing: 2) {
                TextField("name", text: $row.name)
                    .textFieldStyle(.roundedBorder)
                    .font(.body.monospaced())
                    .frame(minWidth: 140, idealWidth: 180, maxWidth: 220, minHeight: 24, maxHeight: 24)
                    .accessibilityLabel("Variable name")
                if let problem {
                    Text(problem)
                        .font(.caption)
                        .foregroundStyle(.orange)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            VariableTextView(
                text: $row.value,
                variables: variables,
                label: "Value of \(row.name.isEmpty ? "new variable" : row.name)",
                environmentName: environmentName,
                singleLine: true
            )
            .frame(minWidth: 220, maxWidth: .infinity, minHeight: 24, maxHeight: 24)

            preview
                .frame(minWidth: 120, maxWidth: 240, minHeight: 24, alignment: .leading)

            Menu {
                if row.isSecretReference {
                    Button("Set Secret Value…", action: onSetSecret)
                }
                Button("Remove Variable", role: .destructive, action: onRemove)
            } label: {
                Image(systemName: "ellipsis.circle")
            }
            .menuStyle(.borderlessButton)
            .menuIndicator(.hidden)
            .fixedSize()
            .frame(minHeight: 24)
            .accessibilityLabel("Actions for \(row.name.isEmpty ? "new variable" : row.name)")
        }
        .contextMenu {
            if row.isSecretReference {
                Button("Set Secret Value…", action: onSetSecret)
            }
            Button("Remove Variable", role: .destructive, action: onRemove)
        }
    }

    @ViewBuilder private var preview: some View {
        if row.isSecretReference {
            Label("Keychain secret", systemImage: "key.fill")
                .font(.caption)
                .foregroundStyle(.purple)
                .help("The value is read from the macOS Keychain when a request is sent")
        } else if let resolved, resolved != row.value {
            Text(resolved)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .help(resolved)
        } else if resolved == nil {
            Text("Not saved yet")
                .font(.caption)
                .foregroundStyle(.tertiary)
                .help("The resolved value appears once the variable is saved")
        } else {
            Text("—")
                .font(.caption)
                .foregroundStyle(.tertiary)
                .accessibilityLabel("Same as value")
        }
    }
}
