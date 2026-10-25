import SwiftUI

/// The capture controls and the live list of exchanges, newest first.
struct CaptureContentView: View {
    @Bindable var capture: CaptureStore
    let workspaceRoot: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            controls
            Divider()
            exchangeList
            Divider()
            footer
        }
    }

    private var controls: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Picker("Browser", selection: $capture.browser) {
                    ForEach(CaptureBrowser.choices, id: \.self) { browser in
                        Text(browser.title).tag(browser)
                    }
                }
                .disabled(capture.isRunning)
                Spacer()
                startStopButton
            }
            TextField("Start URL", text: $capture.startURL, prompt: Text("http://localhost:3000"))
                .textFieldStyle(.roundedBorder)
                .disabled(capture.isRunning)
                .accessibilityLabel("Start URL")
            TextField("Hosts", text: $capture.hostFilter, prompt: Text("All hosts. Or api.example.com, *.local"))
                .textFieldStyle(.roundedBorder)
                .disabled(capture.isRunning)
                .accessibilityLabel("Host filter")
            Toggle("Save each exchange to history", isOn: $capture.saveToHistory)
                .disabled(capture.isRunning)
            if let startError = capture.startError {
                Text(startError)
                    .font(.caption)
                    .foregroundStyle(Color(nsColor: .systemRed))
            }
        }
        .padding(16)
    }

    @ViewBuilder private var startStopButton: some View {
        if capture.isRunning {
            Button(capture.isStopping ? "Stopping…" : "Stop", systemImage: "stop.fill") {
                capture.stop()
            }
            .disabled(capture.isStopping)
            .help("Stop capturing and close the browser")
        } else {
            Button("Start", systemImage: "play.fill") {
                capture.start(workspaceRoot: workspaceRoot)
            }
            .buttonStyle(.borderedProminent)
            .help("Open a separate browser and capture its traffic")
        }
    }

    @ViewBuilder private var exchangeList: some View {
        if capture.exchanges.isEmpty {
            ContentUnavailableView(
                capture.isRunning ? "Waiting for traffic" : "No traffic yet",
                systemImage: "dot.radiowaves.left.and.right",
                description: Text(
                    capture.isRunning
                        ? "Browse in the window Kestrl opened. Requests appear here."
                        : "Start a browser. Kestrl opens a separate window with its own profile."
                )
            )
        } else {
            List(selection: $capture.selectedSeq) {
                ForEach(capture.exchanges.reversed(), id: \.seq) { exchange in
                    CaptureRow(exchange: exchange)
                        .tag(exchange.seq)
                }
            }
        }
    }

    private var footer: some View {
        Text(footerText)
            .font(.caption)
            .foregroundStyle(.secondary)
            .monospacedDigit()
            .lineLimit(1)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
    }

    private var footerText: String {
        if capture.isRunning {
            var text = "\(capture.totalCount) captured"
            if capture.saveToHistory {
                text += " · \(capture.savedCount) saved"
            }
            return text
        }
        return capture.endMessage ?? "Not capturing"
    }
}

/// One exchange in the list: method, status and time on the first line, the path and host below.
private struct CaptureRow: View {
    let exchange: CapturedExchange

    var body: some View {
        let parts = URLParts(exchange.url)
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 8) {
                MethodBadge(method: exchange.method)
                Text(exchange.status.map(String.init) ?? "Failed")
                    .font(.callout.weight(.semibold).monospacedDigit())
                    .foregroundStyle(statusTint)
                Spacer()
                if exchange.eventId != nil {
                    Image(systemName: "tray.and.arrow.down")
                        .foregroundStyle(.secondary)
                        .accessibilityLabel("Saved to history")
                }
                Text("\(exchange.durationMs) ms")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .monospacedDigit()
            }
            Text(parts.path)
                .font(.callout)
                .lineLimit(1)
                .truncationMode(.middle)
            Text(parts.host)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
        .padding(.vertical, 2)
        .accessibilityElement(children: .combine)
    }

    private var statusTint: Color {
        guard let status = exchange.status else { return Color(nsColor: .systemRed) }
        switch status {
        case 200..<300: return Color(nsColor: .systemGreen)
        case 300..<400: return Color(nsColor: .systemBlue)
        case 400..<500: return Color(nsColor: .systemOrange)
        default: return Color(nsColor: .systemRed)
        }
    }
}

/// A URL split into its host and its path with query.
private struct URLParts {
    let host: String
    let path: String

    init(_ text: String) {
        guard let url = URL(string: text), let host = url.host else {
            self.host = ""
            self.path = text
            return
        }
        self.host = host
        let path = url.path.isEmpty ? "/" : url.path
        self.path = url.query.map { "\(path)?\($0)" } ?? path
    }
}
