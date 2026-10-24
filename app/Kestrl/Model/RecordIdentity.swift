// Identity for the FFI records shown in lists and tables.

extension KeyValue: Identifiable {
    public var id: String { key }
}

extension EventSummary: Identifiable {}

extension SecretStatus: Identifiable {
    public var id: String { name }
}
