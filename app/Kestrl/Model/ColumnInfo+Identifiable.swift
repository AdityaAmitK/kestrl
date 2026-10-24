/// Column names are unique within a table, so a column can be its own identity in a `Table`.
extension ColumnInfo: Identifiable {
    public var id: String { name }
}
