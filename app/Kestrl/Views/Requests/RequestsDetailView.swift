import SwiftUI

/// The request on top, its response below. The divider position is remembered, and the response starts with most of the height.
struct RequestsDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VStack(spacing: 0) {
            RequestBar(store: store)
            Divider()
            AutosavingSplit(autosaveName: "kestrl.requests.response.v2", axis: .vertical, defaultFraction: 0.3) {
                requestPane
            } second: {
                responsePane
            }
        }
    }

    private var requestPane: some View {
        RequestPane(store: store)
    }

    @ViewBuilder private var responsePane: some View {
        if store.isSending {
            ProgressView("Sending…")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let response = store.response {
            ResponseView(store: store, response: response)
        } else {
            EmptyPane(
                "No response yet",
                systemImage: "arrow.down.doc",
                description: Text("Send the request to see the response. Press ⌘↩ to send.")
            )
        }
    }
}
