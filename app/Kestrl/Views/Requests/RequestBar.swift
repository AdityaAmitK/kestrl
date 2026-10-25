import SwiftUI

/// Pinned address bar and explicit draft saving; templates stay unresolved in the editor.
struct RequestBar: View {
    @Bindable var store: WorkspaceStore
    @State private var confirmingDiscard = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            AdHocBar(store: store)
            HStack(spacing: 6) {
                Image(systemName: store.selectedRequest == nil ? "square.and.pencil" : "doc.text")
                Text(store.selectedRequest ?? "Unsaved request")
                    .lineLimit(1).truncationMode(.middle)
                if store.isDraftModified { Text("•").help("Unsaved changes") }
                Spacer()
                if store.isDraftModified {
                    Button { confirmingDiscard = true } label: { Image(systemName: "arrow.counterclockwise") }
                        .help("Discard draft changes")
                        .accessibilityLabel("Discard draft changes")
                }
                Button("Save", systemImage: "square.and.arrow.down") {
                    Task { await store.saveRequestDraftAsFile() }
                }
                .disabled(!store.isDraftModified || store.isSending)
                .help("Save the request file (⌘S)")
            }
            .font(.system(size: 11)).foregroundStyle(.secondary).controlSize(.small)
        }
        .padding(.horizontal, 12).padding(.vertical, 10)
        .background(.bar)
        .confirmationDialog("Discard unsaved request changes?", isPresented: $confirmingDiscard) {
            Button("Discard changes", role: .destructive) { Task { await store.discardRequestDraft() } }
        }
    }
}
