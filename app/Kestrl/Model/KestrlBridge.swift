/// Runs the blocking FFI calls on a background thread so the UI never waits on SQLite, files or the network.
enum KestrlBridge {
    static func run<T: Sendable>(_ work: @escaping @Sendable () throws -> T) async throws -> T {
        try await Task.detached(priority: .userInitiated) { try work() }.value
    }
}
