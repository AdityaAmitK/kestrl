import Foundation

/// Event times. Stored as unix milliseconds.
enum TimeFormat {
    /// Local wall-clock time with milliseconds, such as "14:03:07.512". Only used on the main actor.
    private static let clockFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss.SSS"
        return formatter
    }()

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

    /// How long something has run, such as "45s", "3m 12s", "2h 5m" or "4d 3h"
    static func uptime(sinceMs: Int64, now: Date = .now) -> String {
        span(seconds: Int(now.timeIntervalSince(date(ms: sinceMs))))
    }

    /// How long until a moment, in the same form as `uptime`
    static func remaining(untilMs: Int64, now: Date = .now) -> String {
        span(seconds: Int(date(ms: untilMs).timeIntervalSince(now)))
    }

    private static func span(seconds: Int) -> String {
        let seconds = max(0, seconds)
        switch seconds {
        case ..<60: return "\(seconds)s"
        case ..<3_600: return "\(seconds / 60)m \(seconds % 60)s"
        case ..<86_400: return "\(seconds / 3_600)h \(seconds % 3_600 / 60)m"
        default: return "\(seconds / 86_400)d \(seconds % 86_400 / 3_600)h"
        }
    }

    static func absolute(ms: Int64) -> String {
        date(ms: ms).formatted(date: .abbreviated, time: .standard)
    }

    /// "HH:MM:SS.mmm" in local time
    static func clock(ms: Int64) -> String {
        clockFormatter.string(from: date(ms: ms))
    }

    private static func date(ms: Int64) -> Date {
        Date(timeIntervalSince1970: Double(ms) / 1_000)
    }
}
