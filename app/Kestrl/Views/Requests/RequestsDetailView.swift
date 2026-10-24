import SwiftUI

/// The request on top, its response below. The divider position is remembered, and the response starts with most of the height.
struct RequestsDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        AutosavingSplit(autosaveName: "kestrl.requests.response", axis: .vertical, defaultFraction: 0.4) {
            VStack(spacing: 0) {
                RequestBar(store: store)
                Divider()
                requestPane
            }
        } second: {
            responsePane
        }
    }

    @ViewBuilder private var requestPane: some View {
        if store.selectedRequest == nil {
            ContentUnavailableView(
                "New request",
                systemImage: "square.and.pencil",
                description: Text("Enter a URL above and press ⌘↩ to send it. Pick a request on the left to open its file.")
            )
        } else if let detail = store.requestDetail {
            RequestPane(store: store, detail: detail)
        } else {
            Color.clear
        }
    }

    @ViewBuilder private var responsePane: some View {
        if store.isSending {
            ProgressView("Sending…")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        } else if let response = store.response {
            ResponseView(store: store, response: response)
        } else {
            ContentUnavailableView(
                "No response yet",
                systemImage: "arrow.down.doc",
                description: Text("Send the request to see the response. Press ⌘↩ to send.")
            )
        }
    }
}
