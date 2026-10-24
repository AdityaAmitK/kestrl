import SwiftUI

/// The variables of the selected environment, resolved, with secrets masked.
struct EnvironmentDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        if let name = store.inspectedEnvironment {
            VStack(alignment: .leading, spacing: 12) {
                Text(name)
                    .font(.title2.weight(.semibold))
                    .padding(.horizontal, 20)
                    .padding(.top, 20)

                Table(store.resolvedVariables) {
                    TableColumn("Variable") { variable in
                        Text(variable.key)
                            .font(.body.monospaced())
                    }
                    .width(min: 120, ideal: 180)

                    TableColumn("Value") { variable in
                        Text(variable.value)
                            .font(.body.monospaced())
                            .textSelection(.enabled)
                    }
                }
                .overlay {
                    if store.resolvedVariables.isEmpty {
                        ContentUnavailableView("No variables", systemImage: "list.bullet")
                    }
                }

                Text("Secrets are masked. Values are resolved for this environment.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 20)
                    .padding(.bottom, 12)
            }
        } else {
            ContentUnavailableView(
                "Select an environment",
                systemImage: "globe",
                description: Text("Its variables appear here.")
            )
        }
    }
}
