/// What the Database content column has selected: a connection, or one of its tables.
enum DatabaseSelection: Hashable {
    case connection(String)
    case table(DatabaseTable)

    var connection: String {
        switch self {
        case .connection(let name): return name
        case .table(let table): return table.connection
        }
    }
}
