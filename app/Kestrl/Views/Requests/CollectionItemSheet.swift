import SwiftUI

/// What the collection sheet does.
enum CollectionSheetMode: Identifiable, Equatable {
    case newRequest(folder: String)
    case newFolder(parent: String)
    case rename(path: String)

    var id: String {
        switch self {
        case .newRequest(let folder): return "request-\(folder)"
        case .newFolder(let parent): return "folder-\(parent)"
        case .rename(let path): return "rename-\(path)"
        }
    }
}

/// Name a new request or folder (choosing where it goes), or rename one.
struct CollectionItemSheet: View {
    let store: WorkspaceStore
    let mode: CollectionSheetMode

    @Environment(\.dismiss) private var dismiss
    @State private var name = ""
    @State private var folder = ""
    @State private var method = "GET"
    @State private var isWorking = false
    @FocusState private var isNameFocused: Bool

    private static let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]

    private var folders: [String] {
        let all = store.workspace?.folders ?? []
        return all.isEmpty ? ["requests"] : all
    }

    private var trimmedName: String { name.trimmingCharacters(in: .whitespaces) }

    private var problem: String? {
        if trimmedName.isEmpty { return "Enter a name." }
        if trimmedName.contains("/") || trimmedName.hasPrefix(".") { return "Names can't contain / or start with a dot." }
        if case .newFolder = mode {
            let path = folder.isEmpty ? trimmedName : "\(folder)/\(trimmedName)"
            if store.workspace?.folders.contains(path) == true { return "A folder named \"\(trimmedName)\" already exists here." }
        }
        return nil
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.headline)
                Text(explanation)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Form {
                TextField("Name", text: $name, prompt: Text(placeholder))
                    .focused($isNameFocused)
                    .onSubmit(save)
                    .accessibilityLabel("Name")
                switch mode {
                case .newRequest:
                    Picker("Method", selection: $method) {
                        ForEach(Self.methods, id: \.self) { Text($0).tag($0) }
                    }
                    folderPicker(label: "Folder")
                case .newFolder:
                    folderPicker(label: "Inside")
                case .rename:
                    EmptyView()
                }
            }
            .formStyle(.columns)

            Group {
                if let problem, !name.isEmpty {
                    Label(problem, systemImage: "exclamationmark.triangle").foregroundStyle(.orange)
                } else {
                    Text(destinationHint).foregroundStyle(.secondary)
                }
            }
            .font(.caption)

            HStack {
                Spacer()
                Button("Cancel", role: .cancel) { dismiss() }
                    .keyboardShortcut(.cancelAction)
                Button(actionTitle, action: save)
                    .keyboardShortcut(.defaultAction)
                    .disabled(problem != nil || isWorking)
            }
        }
        .padding(20)
        .frame(width: 440)
        .onAppear(perform: prefill)
    }

    private func folderPicker(label: String) -> some View {
        Picker(label, selection: $folder) {
            ForEach(folders, id: \.self) { Text($0).tag($0) }
        }
    }

    private var title: String {
        switch mode {
        case .newRequest: return "New request"
        case .newFolder: return "New folder"
        case .rename(let path): return "Rename \"\(displayName(path))\""
        }
    }

    private var explanation: String {
        switch mode {
        case .newRequest: return "Creates a request file you can edit and send right away. It uses {{api_url}} from the environment."
        case .newFolder: return "A folder groups requests into a collection. Its settings can add variables and pick a default environment."
        case .rename(let path):
            return path.hasSuffix(".req.toml")
                ? "Renames the file. The name shown in the list comes from the request's Options › Name."
                : "Renames the folder. Requests inside move with it."
        }
    }

    private var placeholder: String {
        switch mode {
        case .newRequest: return "List users"
        case .newFolder: return "payments"
        case .rename: return ""
        }
    }

    private var actionTitle: String {
        switch mode {
        case .newRequest, .newFolder: return "Create"
        case .rename: return "Rename"
        }
    }

    private var destinationHint: String {
        switch mode {
        case .newRequest: return "Saved in \(folder.isEmpty ? "the workspace root" : folder + "/")"
        case .newFolder: return "Created as \(folder.isEmpty ? "" : folder + "/")\(trimmedName.isEmpty ? "name" : trimmedName)/"
        case .rename(let path):
            let parent = path.split(separator: "/").dropLast().joined(separator: "/")
            let leaf = path.hasSuffix(".req.toml") ? "\(trimmedName).req.toml" : trimmedName
            return "Becomes \(parent.isEmpty ? "" : parent + "/")\(leaf)"
        }
    }

    private func displayName(_ path: String) -> String {
        let leaf = String(path.split(separator: "/").last ?? "")
        return leaf.hasSuffix(".req.toml") ? String(leaf.dropLast(".req.toml".count)) : leaf
    }

    private func prefill() {
        switch mode {
        case .newRequest(let target):
            folder = target.isEmpty ? (folders.first ?? "requests") : target
        case .newFolder(let parent):
            folder = parent.isEmpty ? (folders.first ?? "requests") : parent
        case .rename(let path):
            name = displayName(path)
        }
        isNameFocused = true
    }

    private func save() {
        guard problem == nil, !isWorking else { return }
        isWorking = true
        let target = trimmedName
        Task {
            let isDone: Bool
            switch mode {
            case .newRequest:
                isDone = await store.createRequest(in: folder, name: target, method: method)
            case .newFolder:
                isDone = await store.createFolder(in: folder, name: target)
            case .rename(let path):
                isDone = await store.renameItem(path, to: target)
            }
            isWorking = false
            if isDone { dismiss() }
        }
    }
}
