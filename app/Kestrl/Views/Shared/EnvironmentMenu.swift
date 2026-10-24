import SwiftUI

/// The toolbar's environment picker. Shows the environment the next send will use.
struct EnvironmentMenu: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        Menu {
            Picker("Environment", selection: $store.selectedEnvironment) {
                ForEach(store.workspace?.envs ?? [], id: \.self) { name in
                    Text(name).tag(String?.some(name))
                }
            }
            .pickerStyle(.inline)
        } label: {
            Label(store.selectedEnvironment ?? "No environment", systemImage: "globe")
        }
        .help("Environment for sends")
        .disabled(store.workspace == nil)
    }
}
