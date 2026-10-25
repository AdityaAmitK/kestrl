import AppKit
import SwiftUI

/// The capture source, its controls for that source only, and the live list of exchanges, newest first.
struct CaptureContentView: View {
    @Bindable var capture: CaptureStore
    let workspaceRoot: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            PaneHeader("Capture", subtitle: subtitle) {
                startStopButton
            }
            Divider()
            controls
            Divider()
            exchangeList
            Divider()
            footer
        }
        .task { capture.refreshReadiness() }
    }

    private var subtitle: String? {
        capture.isRunning ? capture.mode.title : nil
    }

    private var controls: some View {
        VStack(alignment: .leading, spacing: 8) {
            LabeledContent("Source") {
                Picker("Source", selection: $capture.mode) {
                    ForEach(CaptureMode.allCases) { mode in
                        Text(mode.title).tag(mode)
                    }
                }
                .labelsHidden()
                .disabled(capture.isRunning)
            }
            sourceControls
            if capture.mode != .browser {
                LabeledContent("Hosts") {
                    TextField("", text: $capture.hostFilter, prompt: Text("All hosts. Or api.example.com, *.local"))
                        .textFieldStyle(.roundedBorder)
                        .disabled(capture.isRunning)
                        .accessibilityLabel("Host filter")
                }
            }
            LabeledContent("History") {
                Toggle("Save each exchange", isOn: $capture.saveToHistory)
                    .disabled(capture.isRunning)
            }
            if let startError = capture.startError {
                Text(startError)
                    .font(.caption)
                    .foregroundStyle(Color(nsColor: .systemRed))
                    .textSelection(.enabled)
            }
        }
        .controlSize(.small)
        .padding(12)
    }

    @ViewBuilder private var sourceControls: some View {
        switch capture.mode {
        case .browser:
            LabeledContent("Browser") {
                Picker("Browser", selection: $capture.browser) {
                    ForEach(CaptureBrowser.choices, id: \.self) { browser in
                        Text(browser.title).tag(browser)
                    }
                }
                .labelsHidden()
                .disabled(capture.isRunning)
            }
            LabeledContent("Start URL") {
                TextField("", text: $capture.startURL, prompt: Text("http://localhost:3000"))
                    .textFieldStyle(.roundedBorder)
                    .disabled(capture.isRunning)
                    .accessibilityLabel("Start URL")
            }
        case .port:
            LabeledContent("Port") {
                TextField("", text: $capture.portText, prompt: Text("3000"))
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 90)
                    .disabled(capture.isRunning)
                    .accessibilityLabel("Loopback port")
            }
            portSetup
        case .simulator, .android:
            LabeledContent("Proxy port") {
                TextField("", text: $capture.proxyPortText, prompt: Text("9090"))
                    .textFieldStyle(.roundedBorder)
                    .frame(width: 90)
                    .disabled(capture.isRunning)
                    .accessibilityLabel("Proxy port")
            }
            deviceStatus
        }
    }

    /// Port capture needs a one-time, user-run BPF setup. The command is shown and can be copied; Kestrl never runs it.
    @ViewBuilder private var portSetup: some View {
        if let setup = capture.portSetup, !setup.accessible {
            VStack(alignment: .leading, spacing: 6) {
                Text(setup.installed
                    ? "BPF access is not granted to this session yet. If you were just added to kestrl_bpf, log out and back in."
                    : "Port capture needs BPF access. Run this once in Terminal, from a Kestrl checkout. Kestrl does not run it for you.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                if !setup.installed {
                    let command = setup.setupCommand.replacingOccurrences(of: "<your-username>", with: NSUserName())
                    Text(command)
                        .font(.caption.monospaced())
                        .textSelection(.enabled)
                        .lineLimit(2)
                    Button("Copy command") {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(command, forType: .string)
                    }
                }
                Button("Check again") { capture.refreshReadiness() }
            }
        } else if capture.portSetup != nil {
            Label("BPF access is available.", systemImage: "checkmark.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    /// The device must be booted, and its trust set up. The proxy effect is stated here because it changes system settings until Stop.
    private var deviceStatus: some View {
        VStack(alignment: .leading, spacing: 6) {
            if capture.isChecking {
                Label("Checking for a booted device…", systemImage: "hourglass")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else if let problem = capture.deviceProblem {
                Text(problem)
                    .font(.caption)
                    .foregroundStyle(Color(nsColor: .systemOrange))
                    .fixedSize(horizontal: false, vertical: true)
                Button("Check again") { capture.refreshReadiness() }
            } else {
                Label("A device is ready.", systemImage: "checkmark.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Text(proxyEffect)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var proxyEffect: String {
        let port = capture.proxyPortText.isEmpty ? "9090" : capture.proxyPortText
        if capture.mode == .simulator {
            return "Start turns on the macOS web proxy for active network services at 127.0.0.1:\(port) until Stop. Trust Kestrl's root certificate first with `kestrl proxy trust --sim`."
        }
        return "Start sets the emulator's HTTP proxy to 10.0.2.2:\(port) until Stop. Trust Kestrl's root certificate first with `kestrl proxy trust --android`."
    }

    @ViewBuilder private var startStopButton: some View {
        if capture.isRunning {
            Button(capture.isStopping ? "Stopping…" : "Stop", systemImage: "stop.fill") {
                capture.stop()
            }
            .disabled(capture.isStopping)
            .help("Stop capturing and restore any proxy settings")
        } else {
            Button("Start", systemImage: "play.fill") {
                capture.start(workspaceRoot: workspaceRoot)
            }
            .buttonStyle(.borderedProminent)
            .disabled(!capture.canStart)
            .help(startHelp)
        }
    }

    private var startHelp: String {
        switch capture.mode {
        case .browser: return "Open a separate browser and capture its traffic"
        case .port: return "Capture plain HTTP on the loopback port"
        case .simulator, .android: return "Route the device through Kestrl until Stop"
        }
    }

    @ViewBuilder private var exchangeList: some View {
        if capture.exchanges.isEmpty {
            EmptyPane(
                capture.isRunning ? "Waiting for traffic" : "Not capturing",
                systemImage: "dot.radiowaves.left.and.right",
                description: Text(
                    capture.isRunning
                        ? "Requests appear here as they complete."
                        : "Choose a source and press Start."
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
            .padding(.horizontal, 12)
            .padding(.vertical, 6)
    }

    private var footerText: String {
        if capture.isRunning {
            var text = "\(capture.totalCount) captured"
            if capture.saveToHistory {
                text += " · \(capture.savedCount) saved"
            }
            if capture.droppedCount > 0 {
                text += " · \(capture.droppedCount) dropped to stay within memory limits"
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
