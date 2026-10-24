import SwiftUI

/// Copies the shown response as JSON, then confirms briefly.
struct CopyJSONButton: View {
    let store: WorkspaceStore

    var body: some View {
        Button {
            store.copyShownJSON()
        } label: {
            Label(store.justCopied ? "Copied" : "Copy JSON", systemImage: store.justCopied ? "checkmark" : "doc.on.doc")
        }
        .help("Copy the full response as JSON (⇧⌘C)")
        .disabled(!store.hasShownExchange)
    }
}
