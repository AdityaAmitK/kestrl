import SwiftUI

/// The workspace's profiles, with create, duplicate, rename and delete.
struct ProfilesContentView: View {
    @Bindable var store: WorkspaceStore

    private var editor: ProfileEditor { store.profileEditor }
    private var profiles: [String] { store.workspace?.profiles ?? [] }

    var body: some View {
        @Bindable var editor = editor
        VStack(spacing: 0) {
            PaneHeader("Profiles", subtitle: profiles.isEmpty ? nil : "\(profiles.count)") {
                Menu {
                    Button("New Profile…") { editor.nameIntent = .create }
                    if let current = editor.inspected {
                        Button("Duplicate \"\(current)\"…") { editor.nameIntent = .duplicate(source: current) }
                    }
                } label: {
                    Image(systemName: "plus")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .disabled(editor.hasChanges)
                .help(editor.hasChanges ? "Save or revert your edits first" : "New or duplicate profile")
                .accessibilityLabel("Add profile")
            }
            Divider()
            List(selection: selection) {
                ForEach(profiles, id: \.self) { name in
                    row(name)
                        .tag(name)
                        .contextMenu {
                            Group {
                                Button("Duplicate…") { editor.nameIntent = .duplicate(source: name) }
                                Button("Rename…") { editor.nameIntent = .rename(current: name) }
                            }
                            .disabled(editor.hasChanges)
                            Divider()
                            Button("Delete…", role: .destructive) { editor.pendingDeletion = name }
                                .disabled(editor.hasChanges)
                        }
                }
            }
            .listStyle(.sidebar)
            .environment(\.defaultMinListRowHeight, 28)
            .overlay {
                if profiles.isEmpty {
                    EmptyPane(
                        "No profiles",
                        systemImage: "slider.horizontal.3",
                        description: Text("A profile adds headers or query parameters to requests, such as an admin token, either when a request asks for it or automatically for matching URLs.")
                    )
                }
            }
        }
        .sheet(item: $editor.nameIntent) { intent in
            NameSheet(kind: nameKind, intent: intent) { name, copyFrom in
                await editor.commitName(name, intent: intent, copyFrom: copyFrom)
            }
        }
        .confirmationDialog(
            "Delete profile \"\(editor.pendingDeletion ?? "")\"?",
            isPresented: Binding(get: { editor.pendingDeletion != nil }, set: { if !$0 { editor.pendingDeletion = nil } }),
            titleVisibility: .visible
        ) {
            Button("Delete profile", role: .destructive) {
                guard let name = editor.pendingDeletion else { return }
                Task { await editor.delete(name) }
            }
        } message: {
            Text("Requests that list \"\(editor.pendingDeletion ?? "")\" in their profiles fail until it exists again.")
        }
        .alert(
            "Save changes to \"\(editor.loadedProfile ?? "")\"?",
            isPresented: Binding(get: { editor.pendingSelection != nil }, set: { if !$0 { editor.pendingSelection = nil } })
        ) {
            Button("Save") { Task { await switchAfterSaving(true) } }
            Button("Don't Save", role: .destructive) { Task { await switchAfterSaving(false) } }
            Button("Cancel", role: .cancel) { editor.pendingSelection = nil }
        } message: {
            Text("You have unsaved profile edits.")
        }
        .onAppear {
            if editor.inspected == nil { editor.inspected = profiles.first }
        }
        .task(id: editor.inspected) {
            if editor.loadedProfile != editor.inspected || !editor.hasChanges {
                await editor.load(editor.inspected)
            }
        }
    }

    private var selection: Binding<String?> {
        Binding(
            get: { editor.inspected },
            set: { newValue in
                guard newValue != editor.inspected else { return }
                if editor.hasChanges {
                    editor.pendingSelection = newValue
                } else {
                    editor.inspected = newValue
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
        editor.inspected = target
    }

    private func row(_ name: String) -> some View {
        HStack(spacing: 8) {
            Image(systemName: "slider.horizontal.3").foregroundStyle(.secondary)
            Text(name).lineLimit(1)
            Spacer(minLength: 4)
            if editor.inspected == name && editor.hasChanges {
                Circle().fill(.secondary).frame(width: 6, height: 6)
                    .help("Unsaved changes").accessibilityLabel("Unsaved changes")
            }
        }
    }

    private var nameKind: NameSheetKind {
        NameSheetKind(
            noun: "profile",
            validationKind: "environment",
            directory: ".kestrl/profiles",
            existing: profiles,
            explanation: { intent in
                switch intent {
                case .create: return "Headers and query parameters to add to requests, such as X-Admin-Token. Use it with -p, from a request's Options, or automatically for matching URLs."
                case .duplicate: return "The copy keeps every header, parameter, condition and comment."
                case .rename: return "Request files that list the old name are not changed; update their profiles afterwards."
                }
            },
            example: "admin"
        )
    }
}
