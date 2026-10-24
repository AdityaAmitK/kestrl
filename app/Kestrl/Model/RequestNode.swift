import Foundation

/// One row of the Requests outline: a folder, or a request file (a leaf with `request` set).
struct RequestNode: Identifiable {
    let id: String
    let name: String
    let request: RequestEntry?
    let children: [RequestNode]?

    /// Groups request files by folder. Folders come first, then files, each sorted by name.
    static func build(_ entries: [RequestEntry]) -> [RequestNode] {
        let root = FolderBuilder(path: "")
        for entry in entries {
            root.insert(entry)
        }
        return root.nodes()
    }
}

private final class FolderBuilder {
    let path: String
    private var folders: [String: FolderBuilder] = [:]
    private var files: [RequestEntry] = []

    init(path: String) {
        self.path = path
    }

    func insert(_ entry: RequestEntry) {
        let parts = entry.path.split(separator: "/").map(String.init)
        insert(entry, parts: parts[...])
    }

    private func insert(_ entry: RequestEntry, parts: ArraySlice<String>) {
        guard let first = parts.first else { return }
        if parts.count == 1 {
            files.append(entry)
            return
        }
        let child = folders[first] ?? FolderBuilder(path: path.isEmpty ? first : "\(path)/\(first)")
        folders[first] = child
        child.insert(entry, parts: parts.dropFirst())
    }

    func nodes() -> [RequestNode] {
        let folderNodes = folders.sorted { $0.key < $1.key }.map { name, child in
            RequestNode(id: "folder:\(child.path)", name: name, request: nil, children: child.nodes())
        }
        let fileNodes = files.sorted { $0.path < $1.path }.map { entry in
            RequestNode(id: entry.path, name: entry.name ?? Self.stem(of: entry.path), request: entry, children: nil)
        }
        return folderNodes + fileNodes
    }

    private static func stem(of path: String) -> String {
        let file = URL(fileURLWithPath: path).lastPathComponent
        return file.hasSuffix(".req.toml") ? String(file.dropLast(".req.toml".count)) : file
    }
}
