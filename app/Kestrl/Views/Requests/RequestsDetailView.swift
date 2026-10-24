import SwiftUI

/// The ad-hoc bar over a split of the selected request and its response.
struct RequestsDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VSplitView {
            requestPane
                .frame(minHeight: 180)
            responsePane
                .frame(minHeight: 200)
        }
        .safeAreaInset(edge: .top, spacing: 0) {
            AdHocBar(store: store)
        }
    }

    @ViewBuilder private var requestPane: some View {
        if let detail = store.requestDetail {
            RequestPane(store: store, detail: detail)
        } else {
            ContentUnavailableView(
                "Select a request",
                systemImage: "doc.text",
                description: Text("Pick a request on the left, or send an ad-hoc request above.")
            )
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
                description: Text("Send a request to see its response here.")
            )
        }
    }
}
