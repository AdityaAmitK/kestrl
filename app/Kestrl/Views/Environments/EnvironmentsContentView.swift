import SwiftUI

/// The workspace's environments. The default one is marked.
struct EnvironmentsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.inspectedEnvironment) {
            ForEach(store.workspace?.envs ?? [], id: \.self) { name in
                HStack {
                    Label(name, systemImage: "globe")
                    Spacer()
                    if store.workspace?.defaultEnv == name {
                        Text("Default")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                .tag(name)
            }
        }
        .overlay {
            if store.workspace?.envs.isEmpty ?? true {
                ContentUnavailableView(
                    "No environments",
                    systemImage: "globe",
                    description: Text("Add a file under .kestrl/envs/ to define one.")
                )
            }
        }
        .onChange(of: store.inspectedEnvironment) { _, _ in
            Task { await store.loadResolvedVariables() }
        }
    }
}
