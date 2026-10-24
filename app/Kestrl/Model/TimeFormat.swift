import Foundation

/// Event times. Stored as unix milliseconds.
enum TimeFormat {
    /// "just now", "45s ago", "2m ago", "3h ago" or "2d ago"
    static func relative(ms: Int64, now: Date = .now) -> String {
        let seconds = Int(now.timeIntervalSince(date(ms: ms)))
        switch seconds {
        case ..<5: return "just now"
        case ..<60: return "\(seconds)s ago"
        case ..<3_600: return "\(seconds / 60)m ago"
        case ..<86_400: return "\(seconds / 3_600)h ago"
        default: return "\(seconds / 86_400)d ago"
        }
    }

    static func absolute(ms: Int64) -> String {
        date(ms: ms).formatted(date: .abbreviated, time: .standard)
    }

    private static func date(ms: Int64) -> Date {
        Date(timeIntervalSince1970: Double(ms) / 1_000)
    }
}
