import AppKit
import Foundation
import Observation

/// The Capture section. A capture runs on a background thread in `kestrl-ffi`, and exchanges arrive in batches on the main actor.
/// The list keeps the newest `capacity` exchanges within `byteCapacity`, and counts what it drops.
@Observable
@MainActor
final class CaptureStore {
    /// Exchanges kept in the list. Older ones drop off; saving to history keeps them.
    static let capacity = 500
    /// Text kept in the list. Large bodies drop older exchanges sooner.
    static let byteCapacity = 16 * 1024 * 1024
    static let defaultPort = 3000
    static let defaultProxyPort = 9090

    private(set) var exchanges: [CapturedExchange] = []
    /// Exchanges seen since the capture started, including ones dropped before they reached the list
    private(set) var totalCount = 0
    /// Exchanges dropped to stay within the list or buffer limits. Saving to history keeps them.
    private(set) var droppedCount = 0
    /// Why the last capture ended, when it has
    private(set) var endMessage: String?
    /// Why the capture could not start, or why it failed
    private(set) var startError: String?
    private(set) var justCopied = false
    private(set) var isRunning = false
    private(set) var isStopping = false
    /// Why the selected device cannot be captured right now. Nil when ready or not checked.
    private(set) var deviceProblem: String?
    /// BPF access and setup state for port capture. Nil until checked.
    private(set) var portSetup: PortCaptureSetup?
    private(set) var isChecking = false

    /// The source to capture from. The choice is kept between launches.
    var mode: CaptureMode = CaptureMode(rawValue: UserDefaults.standard.string(forKey: CaptureStore.modeKey) ?? "") ?? .browser {
        didSet {
            UserDefaults.standard.set(mode.rawValue, forKey: CaptureStore.modeKey)
            refreshReadiness()
        }
    }

    var browser: CaptureBrowser = CaptureBrowser.choices.first { $0.title == UserDefaults.standard.string(forKey: CaptureStore.browserKey) } ?? .chrome {
        didSet { UserDefaults.standard.set(browser.title, forKey: CaptureStore.browserKey) }
    }
    /// Browser capture only. Not saved between launches, since a start URL can carry credentials.
    var startURL = ""
    /// Comma-separated host globs. Empty captures every host.
    var hostFilter = ""
    /// Port capture: the loopback port the local server listens on
    var portText = "\(CaptureStore.defaultPort)"
    /// Device capture: the local proxy port the device is routed through
    var proxyPortText = "\(CaptureStore.defaultProxyPort)"
    /// Record each exchange in history as it completes
    var saveToHistory = false
    var selectedSeq: UInt64?

    private static let modeKey = "kestrl.capture.mode"
    private static let browserKey = "kestrl.capture.browser"

    private var running: RunningCapture?
    private var retainedBytes = 0
    /// Shown when the running capture ends because of a request to stop, rather than an error
    private var stopMessage = "Capture stopped."
    private var readinessToken = UUID()

    var selectedExchange: CapturedExchange? {
        guard let selectedSeq else { return nil }
        return exchanges.first { $0.seq == selectedSeq }
    }

    var savedCount: Int {
        exchanges.count(where: { $0.eventId != nil })
    }

    /// Whether Start can run now. Device and port setup problems are shown instead of starting a capture that would fail.
    var canStart: Bool {
        switch mode {
        case .browser: return true
        case .port: return portSetup?.accessible ?? false
        case .simulator, .android: return deviceProblem == nil && !isChecking
        }
    }

    /// Check the selected mode's requirements without changing the system. Device checks run off the main thread.
    func refreshReadiness() {
        let token = UUID()
        readinessToken = token
        switch mode {
        case .browser:
            deviceProblem = nil
            portSetup = nil
            isChecking = false
        case .port:
            deviceProblem = nil
            portSetup = portCaptureSetup()
            isChecking = false
        case .simulator, .android:
            portSetup = nil
            isChecking = true
            let mode = mode
            Task {
                let problem: String? = await Task.detached {
                    mode == .simulator ? simulatorCaptureProblem() : androidCaptureProblem()
                }.value
                guard readinessToken == token else { return }
                deviceProblem = problem
                isChecking = false
            }
        }
    }

    /// Start a capture from the chosen source. Exchanges from earlier captures are cleared.
    func start(workspaceRoot: String?) {
        guard running == nil else { return }
        startError = nil
        let id = UUID()
        let relay = CaptureRelay(
            deliver: { [weak self] batch, dropped in
                Task { @MainActor in self?.receive(batch, dropped: dropped, from: id) }
            },
            end: { [weak self] error in
                Task { @MainActor in self?.finish(from: id, error: error) }
            }
        )
        let handle: CaptureHandle
        switch mode {
        case .browser:
            let url = Self.startURL(from: startURL)
            if let url, !Self.isWebURL(url) {
                startError = "The start URL must begin with http:// or https://."
                return
            }
            handle = startBrowserCapture(
                options: BrowserCaptureOptions(
                    workspaceRoot: workspaceRoot,
                    browser: browser,
                    startUrl: url,
                    hosts: Self.hosts(from: hostFilter),
                    saveToHistory: saveToHistory
                ),
                sink: relay
            )
        case .port:
            guard let port = Self.port(from: portText) else {
                startError = "The port must be a number from 1 to 65535."
                return
            }
            handle = startPortCapture(
                options: PortCaptureOptions(
                    workspaceRoot: workspaceRoot,
                    port: port,
                    hosts: Self.hosts(from: hostFilter),
                    saveToHistory: saveToHistory
                ),
                sink: relay
            )
        case .simulator, .android:
            guard let port = Self.port(from: proxyPortText) else {
                startError = "The proxy port must be a number from 1 to 65535."
                return
            }
            handle = startDeviceCapture(
                options: DeviceCaptureOptions(
                    workspaceRoot: workspaceRoot,
                    target: mode == .simulator ? .simulator : .android,
                    port: port,
                    hosts: Self.hosts(from: hostFilter),
                    saveToHistory: saveToHistory
                ),
                sink: relay
            )
        }
        exchanges = []
        retainedBytes = 0
        totalCount = 0
        droppedCount = 0
        selectedSeq = nil
        endMessage = nil
        stopMessage = "Capture stopped."
        running = RunningCapture(id: id, handle: handle, relay: relay)
        isRunning = true
        isStopping = false
    }

    /// Ask the capture to end. It stops once cleanup finishes: the browser is closed, or the device proxy is restored.
    func stop() {
        guard let running, !isStopping else { return }
        isStopping = true
        running.handle.stop()
    }

    /// Stop when the open workspace changes, so exchanges are not saved against the wrong workspace.
    func stopForWorkspaceChange() {
        guard running != nil, !isStopping else { return }
        stopMessage = "The workspace changed, so capture stopped."
        stop()
    }

    /// Stop on quit and wait for cleanup, so the browser is closed and any device proxy is restored before the app exits.
    /// If the wait times out, the device proxy state stays saved and is restored on the next start.
    func stopForQuit() {
        guard let running else { return }
        running.handle.stop()
        _ = running.relay.finished.wait(timeout: .now() + 10)
    }

    func copySelectedJSON() {
        guard let exchange = selectedExchange else { return }
        copyToPasteboard(CaptureJSON.make(exchange))
    }

    private func receive(_ batch: [CapturedExchange], dropped: Int, from id: UUID) {
        guard running?.id == id else { return }
        totalCount += batch.count + dropped
        droppedCount += dropped
        for exchange in batch {
            exchanges.append(exchange)
            retainedBytes += exchange.approximateBytes
        }
        trimToLimits()
    }

    /// Drop the oldest exchanges until the list is within both the count and byte limits.
    private func trimToLimits() {
        var removed = 0
        while exchanges.count > 1 && (exchanges.count > Self.capacity || retainedBytes > Self.byteCapacity) {
            retainedBytes -= exchanges.removeFirst().approximateBytes
            removed += 1
        }
        droppedCount += removed
    }

    private func finish(from id: UUID, error: String?) {
        guard let current = running, current.id == id else { return }
        let wasStopped = isStopping
        running = nil
        isRunning = false
        isStopping = false
        if let error {
            startError = error
            endMessage = nil
        } else {
            endMessage = wasStopped ? stopMessage : "The browser closed, so capture stopped."
        }
        if mode == .simulator || mode == .android {
            refreshReadiness()
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

    /// A TCP port from 1 to 65535. Nil for anything else.
    static func port(from text: String) -> UInt16? {
        guard let value = UInt16(text.trimmingCharacters(in: .whitespaces)), value > 0 else { return nil }
        return value
    }
}

/// Where exchanges come from. Browser capture opens a separate browser; port capture reads a loopback port; device capture routes a simulator or emulator through a local proxy.
enum CaptureMode: String, CaseIterable, Identifiable {
    case browser
    case port
    case simulator
    case android

    var id: Self { self }

    var title: String {
        switch self {
        case .browser: return "Browser"
        case .port: return "Port"
        case .simulator: return "iOS Simulator"
        case .android: return "Android emulator"
        }
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

extension CapturedExchange {
    /// Roughly how much text the list keeps for this exchange: the URL, headers and both bodies.
    var approximateBytes: Int {
        var total = method.utf8.count + url.utf8.count + (requestBody?.utf8.count ?? 0) + (responseBody?.utf8.count ?? 0)
        for pair in requestHeaders + responseHeaders {
            total += pair.key.utf8.count + pair.value.utf8.count
        }
        return total
    }
}

/// The capture the store is following: its id, how to stop it, and where its exchanges come from.
private struct RunningCapture {
    let id: UUID
    let handle: CaptureHandle
    let relay: CaptureRelay
}

/// Receives exchanges on the capture thread and hands them over in batches. A busy source produces many exchanges a second, so the relay flushes about every 150 ms instead of once per exchange.
/// Undelivered exchanges are bounded by bytes: when the buffer is full, the oldest are dropped and counted.
private final class CaptureRelay: CaptureSink, @unchecked Sendable {
    /// Text held between flushes. Bounds memory when the main actor falls behind.
    static let pendingByteLimit = 16 * 1024 * 1024

    /// Signalled when the capture thread has finished, so a quit can wait for cleanup
    let finished = DispatchSemaphore(value: 0)

    private let lock = NSLock()
    private var pending: [CapturedExchange] = []
    private var pendingBytes = 0
    private var droppedPending = 0
    private var flushScheduled = false
    private let deliver: @Sendable ([CapturedExchange], Int) -> Void
    private let end: @Sendable (String?) -> Void

    init(
        deliver: @escaping @Sendable ([CapturedExchange], Int) -> Void,
        end: @escaping @Sendable (String?) -> Void
    ) {
        self.deliver = deliver
        self.end = end
    }

    func onExchange(exchange: CapturedExchange) {
        lock.lock()
        pending.append(exchange)
        pendingBytes += exchange.approximateBytes
        while pendingBytes > Self.pendingByteLimit && pending.count > 1 {
            pendingBytes -= pending.removeFirst().approximateBytes
            droppedPending += 1
        }
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
        let dropped = droppedPending
        pending = []
        pendingBytes = 0
        droppedPending = 0
        flushScheduled = false
        lock.unlock()
        if !batch.isEmpty || dropped > 0 {
            deliver(batch, dropped)
        }
    }
}
