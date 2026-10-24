import Foundation

/// A query result the Database section shows. Cells are decoded once, when the result arrives.
struct ShownQueryResult {
    struct Row: Identifiable {
        let id: Int
        let cells: [QueryCell]
    }

    let columns: [String]
    let rows: [Row]
    let rowCount: UInt64
    let durationMs: UInt64
    let isReadOnly: Bool

    init(_ record: QueryResultRecord) {
        columns = record.columns
        rows = record.rows.enumerated().map { index, cells in
            Row(id: index, cells: cells.map { QueryCell(json: $0) })
        }
        rowCount = record.rowCount
        durationMs = record.durationMs
        isReadOnly = record.isReadOnly
    }

    /// The rows as a JSON array of objects keyed by column name.
    func json() -> String {
        let keys = columns.map { column -> String in
            let data = (try? JSONSerialization.data(withJSONObject: column, options: .fragmentsAllowed)) ?? Data()
            return String(decoding: data, as: UTF8.self)
        }
        let objects = rows.map { row in
            let fields = zip(keys, row.cells).map { "\($0): \($1.json)" }
            return "  {" + fields.joined(separator: ", ") + "}"
        }
        return "[\n" + objects.joined(separator: ",\n") + "\n]"
    }
}
