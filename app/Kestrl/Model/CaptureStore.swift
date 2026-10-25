import AppKit
import Foundation
import Observation

/// Browser capture for the Capture section. The capture runs on a background thread in `kestrl-ffi`. Exchanges arrive in batches on the main actor, and the newest `capacity` are kept.
@Observable
@MainActor
final class CaptureStore {
    /// Exchanges kept in the list. Older ones drop off; saving to history keeps them.
    static let capacity = 500

    private(set) var exchanges: [CapturedExchange] = []
    /// Exchanges seen since the capture started, including ones that have dropped off the list
    private(set) var totalCount = 0
    /// Why the last capture ended, when it has
    private(set) var endMessage: String?
    /// Why the capture could not start
    private(set) var startError: String?
    private(set) var justCopied = false
    private(set) var isRunning = false
    private(set) var isStopping = false

    var browser: CaptureBrowser = .chrome
    var startURL = ""
    /// Comma-separated host globs. Empty captures every host.
    var hostFilter = ""
    /// Record each exchange in history as it completes
    var saveToHistory = false
    var selectedSeq: UInt64?

    private var running: RunningCapture?

    var selectedExchange: CapturedExchange? {
        guard let selectedSeq else { return nil }
        return exchanges.first { $0.seq == selectedSeq }
    }

    var savedCount: Int {
        exchanges.count(where: { $0.eventId != nil })
    }

    /// Start a capture in the chosen browser. Exchanges from earlier captures are cleared.
    func start(workspaceRoot: String?) {
        guard running == nil else { return }
        startError = nil
        let url = Self.startURL(from: startURL)
        if let url, !Self.isWebURL(url) {
            startError = "The start URL must begin with http:// or https://."
            return
        }
        let id = UUID()
        let relay = CaptureRelay(
            deliver: { [weak self] batch in
                Task { @MainActor in self?.receive(batch, from: id) }
            },
            end: { [weak self] error in
                Task { @MainActor in self?.finish(from: id, error: error) }
            }
        )
        let options = BrowserCaptureOptions(
            workspaceRoot: workspaceRoot,
            browser: browser,
            startUrl: url,
            hosts: Self.hosts(from: hostFilter),
            saveToHistory: saveToHistory
        )
        exchanges = []
        totalCount = 0
        selectedSeq = nil
        endMessage = nil
        let handle = startBrowserCapture(options: options, sink: relay)
        running = RunningCapture(id: id, handle: handle, relay: relay)
        isRunning = true
        isStopping = false
    }

    /// Ask the capture to end. It stops once the browser is closed and its profile is removed.
    func stop() {
        guard let running, !isStopping else { return }
        isStopping = true
        running.handle.stop()
    }

    /// Stop on quit and wait briefly, so the browser the capture launched is closed before the app exits.
    func stopForQuit() {
        guard let running else { return }
        running.handle.stop()
        _ = running.relay.finished.wait(timeout: .now() + 5)
    }

    func clear() {
        exchanges = []
        selectedSeq = nil
    }

    func copySelectedJSON() {
        guard let exchange = selectedExchange else { return }
        copyToPasteboard(CaptureJSON.make(exchange))
    }

    private func receive(_ batch: [CapturedExchange], from id: UUID) {
        guard running?.id == id else { return }
        totalCount += batch.count
        exchanges.append(contentsOf: batch)
        if exchanges.count > Self.capacity {
            exchanges.removeFirst(exchanges.count - Self.capacity)
        }
    }

    private func finish(from id: UUID, error: String?) {
        guard let current = running, current.id == id else { return }
        let wasStopped = isStopping
        running = nil
        isRunning = false
        isStopping = false
        if let error {
            endMessage = error
        } else {
            endMessage = wasStopped ? "Capture stopped." : "The browser closed, so capture stopped."
        }
    }

    private func copyToPasteboard(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        justCopied = true
        Task {
            try? await Task.sleep(for: .seconds(1.5))
            justCopied = false
        }
    }

    /// "localhost:3000" becomes "http://localhost:3000". Nil when the field is empty, meaning no start page.
    static func startURL(from text: String) -> String? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty { return nil }
        return trimmed.contains("://") ? trimmed : "http://\(trimmed)"
    }

    static func isWebURL(_ text: String) -> Bool {
        guard let scheme = URL(string: text)?.scheme?.lowercased() else { return false }
        return scheme == "http" || scheme == "https"
    }

    static func hosts(from text: String) -> [String] {
        text.split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }
    }
}

/// The browsers in the picker, in the order they are listed.
extension CaptureBrowser {
    static let choices: [CaptureBrowser] = [.chrome, .arc, .brave, .edge, .chromium, .helium]

    var title: String {
        switch self {
        case .chrome: return "Chrome"
        case .arc: return "Arc"
        case .brave: return "Brave"
        case .edge: return "Edge"
        case .chromium: return "Chromium"
        case .helium: return "Helium"
        }
    }
}

/// The capture the store is following: its id, how to stop it, and where its exchanges come from.
private struct RunningCapture {
    let id: UUID
    let handle: BrowserCapture
    let relay: CaptureRelay
}

/// Receives exchanges on the capture thread and hands them over in batches. A busy page produces many exchanges a second, so the relay flushes about every 150 ms instead of once per exchange.
private final class CaptureRelay: CaptureSink, @unchecked Sendable {
    /// Signalled when the capture thread has finished, so a quit can wait for the browser to close
    let finished = DispatchSemaphore(value: 0)

    private let lock = NSLock()
    private var pending: [CapturedExchange] = []
    private var flushScheduled = false
    private let deliver: @Sendable ([CapturedExchange]) -> Void
    private let end: @Sendable (String?) -> Void

    init(
        deliver: @escaping @Sendable ([CapturedExchange]) -> Void,
        end: @escaping @Sendable (String?) -> Void
    ) {
        self.deliver = deliver
        self.end = end
    }

    func onExchange(exchange: CapturedExchange) {
        lock.lock()
        pending.append(exchange)
        let scheduleFlush = !flushScheduled
        flushScheduled = true
        lock.unlock()
        if scheduleFlush {
            DispatchQueue.global().asyncAfter(deadline: .now() + 0.15) { self.flush() }
        }
    }

    func onEnd(error: String?) {
        flush()
        finished.signal()
        end(error)
    }

    private func flush() {
        lock.lock()
        let batch = pending
        pending = []
        flushScheduled = false
        lock.unlock()
        if !batch.isEmpty {
            deliver(batch)
        }
    }
}
