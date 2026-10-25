import SwiftUI

/// The selected exchange: its status and timing, then the request or response as body or headers, with Copy JSON.
struct CaptureDetailView: View {
    @Bindable var capture: CaptureStore
    let store: WorkspaceStore
    @State private var side: Side = .response
    @State private var part: Part = .body
    @State private var preparedBody: PreparedBody?

    private enum Side: String, CaseIterable, Identifiable {
        case request = "Request"
        case response = "Response"

        var id: Self { self }
    }

    private enum Part: String, CaseIterable, Identifiable {
        case body = "Body"
        case headers = "Headers"

        var id: Self { self }
    }

    var body: some View {
        if let exchange = capture.selectedExchange {
            VStack(alignment: .leading, spacing: 0) {
                header(for: exchange)
                Divider()
                HStack {
                    Picker("Side", selection: $side) {
                        ForEach(Side.allCases) { option in
                            Text(option.rawValue).tag(option)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .fixedSize()
                    Picker("Show", selection: $part) {
                        ForEach(Part.allCases) { option in
                            Text(option.rawValue).tag(option)
                        }
                    }
                    .pickerStyle(.segmented)
                    .labelsHidden()
                    .fixedSize()
                    Spacer()
                }
                .controlSize(.small)
                .padding(8)
                content(for: exchange)
            }
            .task(id: "\(exchange.seq)-\(side.rawValue)") {
                preparedBody = nil
                let selection = exchange.seq
                let desiredSide = side
                let raw = desiredSide == .request ? exchange.requestBody : exchange.responseBody
                if let raw {
                    let prepared = await PreparedBody.prepare(raw)
                    guard !Task.isCancelled, capture.selectedSeq == selection, side == desiredSide else { return }
                    preparedBody = prepared
                }
            }
        } else {
            EmptyPane(
                "Select an exchange",
                systemImage: "dot.radiowaves.left.and.right",
                description: Text("Its headers, body and status appear here.")
            )
        }
    }

    private func header(for exchange: CapturedExchange) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                MethodBadge(method: exchange.method)
                Text(exchange.url)
                    .font(.headline)
                    .textSelection(.enabled)
                    .lineLimit(2)
                Spacer()
                Button("Use as request", systemImage: "square.and.pencil") {
                    store.useCapturedRequest(exchange)
                }
                .help("Open an editable request draft; sending remains explicit")
                Button {
                    capture.copySelectedJSON()
                } label: {
                    Label(capture.justCopied ? "Copied" : "Copy JSON", systemImage: capture.justCopied ? "checkmark" : "doc.on.doc")
                }
                .help("Copy the request and response as JSON")
            }
            HStack(spacing: 12) {
                StatusBadge(status: exchange.status, error: exchange.status == nil ? "Failed" : nil)
                Label("\(exchange.durationMs) ms", systemImage: "timer")
                Text(TimeFormat.clock(ms: exchange.tsMs))
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
                if exchange.eventId != nil {
                    Label("Saved to history", systemImage: "tray.and.arrow.down")
                        .foregroundStyle(.secondary)
                }
            }
            .font(.callout)
            if let historyError = exchange.historyError {
                Text("Not saved to history: \(historyError)")
                    .font(.caption)
                    .foregroundStyle(Color(nsColor: .systemOrange))
                    .textSelection(.enabled)
            }
        }
        .padding(12)
    }

    @ViewBuilder private func content(for exchange: CapturedExchange) -> some View {
        switch part {
        case .body:
            if let preparedBody, !preparedBody.text.isEmpty {
                JSONTextView(text: preparedBody.text)
            } else {
                EmptyPane("No body", systemImage: "doc.plaintext")
            }
        case .headers:
            let headers = side == .request ? exchange.requestHeaders : exchange.responseHeaders
            if headers.isEmpty {
                EmptyPane("No headers", systemImage: "list.bullet.rectangle")
            } else {
                ScrollView {
                    KeyValueGrid(title: side.rawValue, pairs: headers)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(12)
                }
            }
        }
    }
}
