import SwiftUI

/// The collection outline: folders and request files, searchable, with create / rename /
/// duplicate / delete and per-folder settings.
struct RequestsContentView: View {
    @Bindable var store: WorkspaceStore
    @State private var search = ""
    @State private var sheet: CollectionSheetMode?
    @State private var settingsFolder: FolderName?
    @State private var pendingTrash: String?

    /// Wraps a folder path so it can drive `.sheet(item:)`.
    private struct FolderName: Identifiable {
        let path: String
        var id: String { path }
    }

    private var entries: [RequestEntry] {
        (store.workspace?.requests ?? []).filter {
            search.isEmpty || ($0.name ?? $0.path).localizedCaseInsensitiveContains(search)
                || $0.path.localizedCaseInsensitiveContains(search)
        }
    }

    /// The folder new items go into: the selected request's folder, else the first collection.
    private var currentFolder: String {
        if let selected = store.selectedRequest, selected.contains("/") {
            return selected.split(separator: "/").dropLast().joined(separator: "/")
        }
        return store.workspace?.folders.first ?? "requests"
    }

    var body: some View {
        VStack(spacing: 0) {
            PaneHeader("Collection", subtitle: "\(store.workspace?.requests.count ?? 0)") {
                Menu {
                    Button("New Request…") { sheet = .newRequest(folder: currentFolder) }
                    Button("New Folder…") { sheet = .newFolder(parent: currentFolder) }
                    Divider()
                    Button("New Unsaved Request") { store.newRequest() }
                } label: {
                    Image(systemName: "plus")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .fixedSize()
                .help("New request or folder")
                .accessibilityLabel("Add to collection")
            }
            Divider()
            TextField("Find a request", text: $search)
                .textFieldStyle(.roundedBorder).controlSize(.small)
                .padding(8).accessibilityLabel("Find a request")
            List(selection: $store.selectedRequest) {
                RequestTreeRows(
                    nodes: RequestNode.build(entries, folders: search.isEmpty ? (store.workspace?.folders ?? []) : []),
                    store: store,
                    isFiltering: !search.isEmpty,
                    onAction: perform
                )
            }
            .listStyle(.plain)
            .environment(\.defaultMinListRowHeight, 26)
            .overlay {
                if entries.isEmpty && (!search.isEmpty || (store.workspace?.folders.isEmpty ?? true)) {
                    EmptyPane(search.isEmpty ? "No requests" : "No matches", systemImage: "doc.text",
                              description: Text(search.isEmpty ? "Create a request or a folder with +." : "Try a different name."))
                }
            }
        }
        .sheet(item: $sheet) { mode in
            CollectionItemSheet(store: store, mode: mode)
        }
        .sheet(item: $settingsFolder) { folder in
            FolderSettingsSheet(store: store, folder: folder.path)
        }
        .confirmationDialog(
            "Move \"\(pendingTrash.map(displayName) ?? "")\" to the Trash?",
            isPresented: Binding(get: { pendingTrash != nil }, set: { if !$0 { pendingTrash = nil } }),
            titleVisibility: .visible
        ) {
            Button("Move to Trash", role: .destructive) {
                guard let path = pendingTrash else { return }
                Task { await store.trashItem(path) }
            }
        } message: {
            Text(pendingTrash?.hasSuffix(".req.toml") == true
                 ? "You can restore it from the Trash in Finder."
                 : "The folder and every request in it move to the Trash. You can restore them in Finder.")
        }
        .onChange(of: store.selectedRequest) { _, _ in
            Task { await store.requestSelectionChanged() }
        }
    }

    private func perform(_ action: CollectionAction) {
        switch action {
        case .newRequest(let folder): sheet = .newRequest(folder: folder)
        case .newFolder(let parent): sheet = .newFolder(parent: parent)
        case .settings(let folder): settingsFolder = FolderName(path: folder)
        case .rename(let path): sheet = .rename(path: path)
        case .duplicate(let path): Task { await store.duplicateRequest(path) }
        case .delete(let path): pendingTrash = path
        case .reveal(let path): store.revealInFinder(path)
        }
    }

    private func displayName(_ path: String) -> String {
        let leaf = String(path.split(separator: "/").last ?? "")
        return leaf.hasSuffix(".req.toml") ? String(leaf.dropLast(".req.toml".count)) : leaf
    }
}
