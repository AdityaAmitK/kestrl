import SwiftUI

/// The workspace's environments, with create, duplicate, rename, default and delete.
struct EnvironmentsContentView: View {
    @Bindable var store: WorkspaceStore

    private var editor: EnvironmentEditor { store.environmentEditor }
    private var environments: [String] { store.workspace?.envs ?? [] }
    private var defaultEnvironment: String? { store.workspace?.defaultEnv }

    var body: some View {
        @Bindable var editor = editor
        VStack(spacing: 0) {
            PaneHeader("Environments", subtitle: environments.isEmpty ? nil : "\(environments.count)") {
                addMenu
            }
            Divider()
            List(selection: selection) {
                ForEach(environments, id: \.self) { name in
                    row(name)
                        .tag(name)
                        .contextMenu { contextMenu(for: name) }
                }
            }
            .listStyle(.sidebar)
            .environment(\.defaultMinListRowHeight, 28)
            .overlay {
                if environments.isEmpty {
                    EmptyPane(
                        "No environments",
                        systemImage: "globe",
                        description: Text("Environments hold the variables your requests use, such as api_url for local and production.")
                    )
                }
            }
        }
        .sheet(item: $editor.nameIntent) { intent in
            EnvironmentNameSheet(store: store, intent: intent)
        }
        .confirmationDialog(
            "Delete \"\(editor.pendingDeletion ?? "")\"?",
            isPresented: Binding(get: { editor.pendingDeletion != nil }, set: { if !$0 { editor.pendingDeletion = nil } }),
            titleVisibility: .visible
        ) {
            Button("Delete environment", role: .destructive) {
                guard let name = editor.pendingDeletion else { return }
                Task { await editor.delete(name) }
            }
        } message: {
            Text("Its file .kestrl/envs/\(editor.pendingDeletion ?? "").toml is removed. Secrets in the Keychain are not touched.")
        }
        .alert(
            "Save changes to \"\(editor.loadedEnvironment ?? "")\"?",
            isPresented: Binding(get: { editor.pendingSelection != nil }, set: { if !$0 { editor.pendingSelection = nil } })
        ) {
            Button("Save") { Task { await switchAfterSaving(true) } }
            Button("Don't Save", role: .destructive) { Task { await switchAfterSaving(false) } }
            Button("Cancel", role: .cancel) { editor.pendingSelection = nil }
        } message: {
            Text("You have unsaved variable edits.")
        }
        .task(id: store.inspectedEnvironment) {
            if editor.loadedEnvironment != store.inspectedEnvironment || !editor.hasChanges {
                await editor.load(store.inspectedEnvironment)
            }
        }
    }

    /// Switching away from unsaved edits asks first instead of dropping them.
    private var selection: Binding<String?> {
        Binding(
            get: { store.inspectedEnvironment },
            set: { newValue in
                guard newValue != store.inspectedEnvironment else { return }
                if editor.hasChanges {
                    editor.pendingSelection = newValue
                } else {
                    store.inspectedEnvironment = newValue
                }
            }
        )
    }

    private func switchAfterSaving(_ shouldSave: Bool) async {
        guard let target = editor.pendingSelection else { return }
        editor.pendingSelection = nil
        if shouldSave {
            guard await editor.save() else { return }
        } else {
            editor.revert()
        }
        store.inspectedEnvironment = target
    }

    private func row(_ name: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "globe")
                .foregroundStyle(.secondary)
            Text(name)
                .lineLimit(1)
            Spacer(minLength: 4)
            if defaultEnvironment == name {
                Text("Default")
                    .font(.caption2.weight(.medium))
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 6)
                    .padding(.vertical, 1)
                    .background(.quaternary, in: Capsule())
                    .accessibilityLabel("Default environment")
            }
            if store.inspectedEnvironment == name && editor.hasChanges {
                Circle()
                    .fill(.secondary)
                    .frame(width: 6, height: 6)
                    .help("Unsaved changes")
                    .accessibilityLabel("Unsaved changes")
            }
        }
    }

    private var addMenu: some View {
        Menu {
            Button("New Environment…") { editor.nameIntent = .create }
            if let current = store.inspectedEnvironment {
                Button("Duplicate \"\(current)\"…") { editor.nameIntent = .duplicate(source: current) }
            }
            if editor.hasChanges {
                Divider()
                Text("Save or revert your edits first")
            }
        } label: {
            Image(systemName: "plus")
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .fixedSize()
        .disabled(editor.hasChanges)
        .help(editor.hasChanges ? "Save or revert your edits first" : "New or duplicate environment")
        .accessibilityLabel("Add environment")
    }

    @ViewBuilder private func contextMenu(for name: String) -> some View {
        // Actions that reload the editor wait until unsaved edits are saved or reverted.
        Group {
            Button("Duplicate…") { editor.nameIntent = .duplicate(source: name) }
            Button("Rename…") { editor.nameIntent = .rename(current: name) }
        }
        .disabled(editor.hasChanges)
        Divider()
        Button("Make Default") { Task { await editor.makeDefault(name) } }
            .disabled(defaultEnvironment == name)
        Divider()
        Button(defaultEnvironment == name ? "Delete… (make another default first)" : "Delete…", role: .destructive) {
            editor.pendingDeletion = name
        }
        .disabled(defaultEnvironment == name || editor.hasChanges)
    }
}
