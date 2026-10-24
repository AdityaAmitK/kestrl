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
        .labelStyle(.titleAndIcon)
        .menuIndicator(.visible)
        .help("Environment for sends")
        .accessibilityLabel("Environment: \(store.selectedEnvironment ?? "none")")
        .disabled(store.workspace == nil)
    }
}
