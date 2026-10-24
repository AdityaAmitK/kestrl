import SwiftUI

/// The request as written in its file, with variables unresolved, and the Send button.
struct RequestPane: View {
    let store: WorkspaceStore
    let detail: RequestDetail

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    MethodBadge(method: detail.method)
                    Text(detail.name ?? store.selectedRequest ?? "")
                        .font(.title3.weight(.semibold))
                        .lineLimit(1)
                    Spacer()
                    Button {
                        Task { await store.sendSelectedRequest() }
                    } label: {
                        Label(store.isSending ? "Sending…" : "Send", systemImage: "paperplane.fill")
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(store.isSending)
                    .help("Send the request (⌘↩)")
                }

                VStack(alignment: .leading, spacing: 4) {
                    Text(detail.url)
                        .font(.body.monospaced())
                        .textSelection(.enabled)
                    Text(store.selectedRequest ?? "")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }

                if !detail.profiles.isEmpty {
                    Label(detail.profiles.joined(separator: ", "), systemImage: "square.stack.3d.up")
                        .foregroundStyle(.secondary)
                }

                KeyValueGrid(title: "Query", pairs: detail.query)
                KeyValueGrid(title: "Headers", pairs: detail.headers)

                if let body = store.requestBody {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Body")
                            .font(.headline)
                        JSONTextView(text: body.text)
                            .frame(minHeight: 120, maxHeight: 220)
                            .clipShape(RoundedRectangle(cornerRadius: 6))
                            .overlay(RoundedRectangle(cornerRadius: 6).strokeBorder(.separator))
                    }
                }
            }
            .padding(20)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
