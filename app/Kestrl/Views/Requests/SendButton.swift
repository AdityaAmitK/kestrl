import SwiftUI

/// The prominent Send control. Sends the open request, or the ad-hoc URL when no request file is open. ⌘↩ is registered in the Request menu.
struct SendButton: View {
    let store: WorkspaceStore

    var body: some View {
        Button {
            Task { await store.sendCurrentRequest() }
        } label: {
            Label(store.isSending ? "Sending…" : "Send", systemImage: "paperplane.fill")
                .frame(minWidth: 64)
        }
        .buttonStyle(.borderedProminent)
        .controlSize(.large)
        .disabled(!store.canSendRequest)
        .help("Send the request (⌘↩)")
    }
}
