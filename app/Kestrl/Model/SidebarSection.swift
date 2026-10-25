/// The sidebar sections. The raw value is what state restoration stores.
enum SidebarSection: String, CaseIterable, Identifiable, Hashable {
    case requests
    case history
    case capture
    case logs
    case database
    case environments
    case secrets

    var id: Self { self }

    var title: String {
        switch self {
        case .requests: return "Requests"
        case .history: return "History"
        case .capture: return "Capture"
        case .logs: return "Logs"
        case .database: return "Database"
        case .environments: return "Environments"
        case .secrets: return "Secrets"
        }
    }

    var symbol: String {
        switch self {
        case .requests: return "doc.text"
        case .history: return "clock.arrow.circlepath"
        case .capture: return "dot.radiowaves.left.and.right"
        case .logs: return "text.alignleft"
        case .database: return "cylinder.split.1x2"
        case .environments: return "globe"
        case .secrets: return "key"
        }
    }
}
