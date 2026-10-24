import SwiftUI

/// The last response: status, timing, size, masked secrets, and the body or headers.
struct ResponseView: View {
    let store: WorkspaceStore
    let response: ShownResponse

    var body: some View {
        let exchange = response.exchange
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 14) {
                StatusBadge(status: exchange.status, error: exchange.error)
                Text("\(exchange.durationMs) ms")
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                Text(ByteCountFormatter.string(fromByteCount: Int64(exchange.body.utf8.count), countStyle: .file))
                    .monospacedDigit()
                    .foregroundStyle(.secondary)
                if !exchange.redacted.isEmpty {
                    Label(maskedText(count: exchange.redacted.count), systemImage: "eye.slash")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .help(exchange.redacted.joined(separator: ", "))
                }
                if let historyError = exchange.historyError {
                    Label("Not saved to history", systemImage: "exclamationmark.triangle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .help(historyError)
                }
                Spacer()
                CopyJSONButton(store: store)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)

            Divider()

            ExchangeInspector(
                headers: exchange.headers,
                payload: response.body,
                emptyMessage: exchange.error ?? "The response had no body."
            )
        }
    }

    private func maskedText(count: Int) -> String {
        count == 1 ? "1 secret masked" : "\(count) secrets masked"
    }
}
