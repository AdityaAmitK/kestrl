/// The sidebar sections. The raw value is what state restoration stores.
enum SidebarSection: String, CaseIterable, Identifiable, Hashable {
    case requests
    case history
    case environments
    case secrets

    var id: Self { self }

    var title: String {
        switch self {
        case .requests: return "Requests"
        case .history: return "History"
        case .environments: return "Environments"
        case .secrets: return "Secrets"
        }
    }

    var symbol: String {
        switch self {
        case .requests: return "doc.text"
        case .history: return "clock.arrow.circlepath"
        case .environments: return "globe"
        case .secrets: return "key"
        }
    }
}
