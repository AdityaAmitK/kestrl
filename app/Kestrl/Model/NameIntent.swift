/// How a name sheet (environment, profile or connection) was opened.
enum NameIntent: Identifiable, Equatable {
    case create
    case duplicate(source: String)
    case rename(current: String)

    var id: String {
        switch self {
        case .create: return "create"
        case .duplicate(let source): return "duplicate-\(source)"
        case .rename(let current): return "rename-\(current)"
        }
    }
}
