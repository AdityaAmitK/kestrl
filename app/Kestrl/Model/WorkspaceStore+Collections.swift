import AppKit
import Foundation

/// Creating, renaming, duplicating and deleting collection folders and request files.
/// Deletes move items to the Trash, so they can be recovered.
extension WorkspaceStore {
    /// Creates a request named `name` in `folder` and opens it.
    func createRequest(in folder: String, name: String, method: String) async -> Bool {
        guard let root = workspace?.root else { return false }
        do {
            let path = try await KestrlBridge.run {
                try Kestrl.createRequest(workspaceRoot: root, folder: folder, name: name, method: method)
            }
            await reloadCollection(revealing: path)
            selectedRequest = path
            await requestSelectionChanged()
            return true
        } catch {
            report(error)
            return false
        }
    }

    func createFolder(in parent: String, name: String) async -> Bool {
        guard let root = workspace?.root else { return false }
        let folder = parent.isEmpty ? name : "\(parent)/\(name)"
        do {
            try await KestrlBridge.run { try Kestrl.createFolder(workspaceRoot: root, folder: folder) }
            await reloadCollection(revealing: folder + "/")
            return true
        } catch {
            report(error)
            return false
        }
    }

    func duplicateRequest(_ path: String) async {
        guard let root = workspace?.root else { return }
        do {
            let copy = try await KestrlBridge.run { try Kestrl.duplicateRequest(workspaceRoot: root, path: path) }
            await reloadCollection(revealing: copy)
            selectedRequest = copy
            await requestSelectionChanged()
        } catch {
            report(error)
        }
    }

    /// Renames a folder, or a request file (its file name; the display name lives in the file).
    func renameItem(_ path: String, to newName: String) async -> Bool {
        guard let root = workspace?.root else { return false }
        let parent = path.split(separator: "/").dropLast().joined(separator: "/")
        let isRequest = path.hasSuffix(".req.toml")
        let leaf = isRequest && !newName.hasSuffix(".req.toml") ? "\(newName).req.toml" : newName
        let target = parent.isEmpty ? leaf : "\(parent)/\(leaf)"
        guard target != path else { return true }
        do {
            try await KestrlBridge.run { try Kestrl.moveItem(workspaceRoot: root, from: path, to: target) }
            if selectedRequest == path || (selectedRequest?.hasPrefix(path + "/") ?? false) {
                selectedRequest = selectedRequest.map { target + $0.dropFirst(path.count) }
            }
            await reloadCollection(revealing: target)
            await requestSelectionChanged()
            return true
        } catch {
            report(error)
            return false
        }
    }

    /// Moves a request file or folder to the Trash.
    func trashItem(_ path: String) async {
        guard let root = workspace?.root else { return }
        let url = URL(fileURLWithPath: root).appendingPathComponent(path)
        do {
            try FileManager.default.trashItem(at: url, resultingItemURL: nil)
            let wasOpen = selectedRequest == path || (selectedRequest?.hasPrefix(path + "/") ?? false)
            if wasOpen { selectedRequest = nil }
            await reloadCollection(revealing: nil)
            // Clears the editor when the open request was trashed.
            if wasOpen { await requestSelectionChanged() }
        } catch {
            report(error)
        }
    }

    func revealInFinder(_ path: String) {
        guard let root = workspace?.root else { return }
        NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: root).appendingPathComponent(path)])
    }

    /// Re-reads the workspace and makes sure the folders leading to `path` are expanded.
    private func reloadCollection(revealing path: String?) async {
        if let path {
            let parts = path.split(separator: "/").map(String.init)
            for count in 1..<max(parts.count, 1) {
                setFolder("folder:" + parts[..<count].joined(separator: "/"), expanded: true)
            }
            if path.hasSuffix("/") {
                setFolder("folder:" + String(path.dropLast()), expanded: true)
            }
        }
        guard let root = workspace?.root else { return }
        await loadWorkspace(from: URL(fileURLWithPath: root))
    }
}
