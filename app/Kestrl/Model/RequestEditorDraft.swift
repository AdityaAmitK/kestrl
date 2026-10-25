import Foundation

/// An unresolved request draft. Switching rows keeps edits in memory until explicitly saved.
struct RequestEditorDraft: Equatable {
    var name = ""
    var method = "GET"
    var url = ""
    var headers = ""
    var query = ""
    var bodyKind = "none"
    var body = ""
    var profiles = ""
    var sourceText = ""

    init() {}

    init(_ detail: RequestDetail) {
        name = detail.name ?? ""
        method = detail.method
        url = detail.url
        headers = detail.headers.map { "\($0.key): \($0.value)" }.joined(separator: "\n")
        query = detail.query.map { "\($0.key)=\($0.value)" }.joined(separator: "\n")
        bodyKind = detail.bodyKind ?? "none"
        body = detail.body ?? ""
        profiles = detail.profiles.joined(separator: ", ")
        sourceText = detail.sourceText
    }

    func record() throws -> RequestDetail {
        RequestDetail(sourceText: sourceText, name: name.isEmpty ? nil : name, method: method,
                      url: url, profiles: profiles.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty },
                      headers: try pairs(headers, separator: ":"), query: try pairs(query, separator: "="),
                      bodyKind: bodyKind == "none" ? nil : bodyKind, body: bodyKind == "none" ? nil : body)
    }

    private func pairs(_ text: String, separator: Character) throws -> [KeyValue] {
        try text.split(separator: "\n").filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }.map { line in
            guard let divider = line.firstIndex(of: separator) else {
                throw KestrlError.InvalidInput(message: "Each line must use name\(separator)value.")
            }
            let key = line[..<divider].trimmingCharacters(in: .whitespaces)
            guard !key.isEmpty else { throw KestrlError.InvalidInput(message: "Names cannot be empty.") }
            let rawValue = String(line[line.index(after: divider)...])
            return KeyValue(key: key, value: separator == ":" ? rawValue.trimmingCharacters(in: .whitespaces) : rawValue)
        }
    }
}
