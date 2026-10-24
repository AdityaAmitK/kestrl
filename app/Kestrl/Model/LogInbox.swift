import Foundation

/// Holds log lines as the core emits them, on its own thread. The main actor drains it every 100 ms, so the UI updates in batches rather than once per line.
final class LogInbox: LogSink, @unchecked Sendable {
    private let lock = NSLock()
    private var pending: [LogLine] = []
    private var hasEnded = false
    private var endError: String?

    func onLine(line: LogLine) {
        lock.withLock { pending.append(line) }
    }

    func onEnd(error: String?) {
        lock.withLock {
            hasEnded = true
            endError = error
        }
    }

    /// Takes the lines received so far, and whether the stream has ended.
    func drain() -> (lines: [LogLine], ended: Bool, error: String?) {
        lock.withLock {
            let lines = pending
            pending = []
            return (lines, hasEnded, endError)
        }
    }
}
