import Foundation

/// Checks a log filter before the core sees it. The core uses Rust regex syntax; NSRegularExpression accepts nearly the same patterns, so typos show inline instead of as a failed stream.
enum LogFilter {
    /// The message to show under the filter field, or nil when the pattern is usable. An empty pattern filters nothing.
    static func problem(with pattern: String) -> String? {
        guard !pattern.isEmpty else { return nil }
        do {
            _ = try NSRegularExpression(pattern: pattern)
            return nil
        } catch {
            return "Not a valid regular expression."
        }
    }
}
