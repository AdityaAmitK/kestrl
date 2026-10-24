import SwiftUI

/// Settings for the open workspace. Read-only until settings are editable in the app.
struct SettingsView: View {
    let store: WorkspaceStore

    var body: some View {
        Form {
            if let settings = store.workspace?.settings {
                Section("History") {
                    LabeledContent("Database") {
                        Text(settings.historyDb)
                            .textSelection(.enabled)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    LabeledContent("Keep events for", value: "\(settings.maxAgeDays) days")
                    LabeledContent("Keep at most", value: "\(settings.maxEvents) events")
                    LabeledContent("Cut bodies to") {
                        Text(ByteCountFormatter.string(fromByteCount: Int64(settings.maxBodyBytes), countStyle: .file))
                    }
                }
                Section {
                    Text("Retention is set in .kestrl/kestrl.toml under [retention].")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            } else {
                Text("Open a workspace to see its settings.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .frame(width: 520)
        .scenePadding()
    }
}
