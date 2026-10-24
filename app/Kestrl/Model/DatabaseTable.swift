/// A table in a connection, identified by the connection name, schema and table name.
struct DatabaseTable: Hashable {
    let connection: String
    let schema: String
    let name: String

    /// The schema-qualified name the core expects, such as `public.users`.
    var qualifiedName: String { "\(schema).\(name)" }
}
