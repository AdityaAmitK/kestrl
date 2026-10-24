import Foundation

/// One value from a query result. The core sends each cell as JSON, so the exact value can be copied and NULL can be told apart from the text "null".
struct QueryCell {
    /// The longest text shown in a cell. Longer values are cut here and shown in full on hover.
    private static let previewLength = 200

    /// The value as JSON, for Copy JSON.
    let json: String
    /// The full value as text, for hover and copy.
    let text: String
    /// The first `previewLength` characters of `text`, with an ellipsis when cut.
    let preview: String
    let isNull: Bool

    init(json: String) {
        self.json = json
        isNull = json == "null"
        if isNull {
            text = "NULL"
        } else if json.hasPrefix("\""),
                  let decoded = try? JSONDecoder().decode(String.self, from: Data(json.utf8)) {
            text = decoded
        } else {
            text = json
        }
        preview = text.count > Self.previewLength
            ? String(text.prefix(Self.previewLength)) + "…"
            : text
    }
}
