import SwiftUI

/// The top bar of the Requests section. With a request file open it shows that request's URL template; otherwise it is the ad-hoc bar.
struct RequestBar: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if store.selectedRequest == nil {
                AdHocBar(store: store)
            } else if let detail = store.requestDetail {
                HStack(spacing: 12) {
                    MethodBadge(method: detail.method)
                    Text(detail.url)
                        .font(.body.monospaced())
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                    SendButton(store: store)
                }
            } else {
                ProgressView()
                    .controlSize(.small)
                    .frame(maxWidth: .infinity, minHeight: 28, alignment: .leading)
            }

            Text(environmentHint)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(.bar)
    }

    private var environmentHint: String {
        var hint = "Environment: \(store.selectedEnvironment ?? "none")"
        if let profiles = store.requestDetail?.profiles, !profiles.isEmpty {
            hint += " · Profiles: \(profiles.joined(separator: ", "))"
        }
        return hint
    }
}
