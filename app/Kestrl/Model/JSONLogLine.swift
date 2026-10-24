import Foundation

/// A log line that is a JSON object with `level` and `message`, split into the parts the log view shows.
/// Any other line is plain text and is shown as written.
struct JSONLogLine {
    /// One remaining key of the object, shown as `key=value`
    struct Field: Identifiable {
        let key: String
        let value: String

        var id: String { key }
    }

    let level: String
    let message: String
    /// The other keys, sorted by name so the order stays the same from line to line
    let fields: [Field]

    /// nil unless `text` is a JSON object that has both `level` and `message`.
    init?(text: String) {
        guard let object = try? JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any],
              let level = object["level"],
              let message = object["message"]
        else { return nil }
        self.level = Self.string(from: level)
        self.message = Self.string(from: message)
        fields = object
            .filter { $0.key != "level" && $0.key != "message" }
            .sorted { $0.key < $1.key }
            .map { Field(key: $0.key, value: Self.string(from: $0.value)) }
    }

    /// Text values as they are; any other JSON value as compact JSON, such as `42`, `true` or `{"a":1}`.
    private static func string(from value: Any) -> String {
        if let text = value as? String { return text }
        let data = (try? JSONSerialization.data(withJSONObject: value, options: .fragmentsAllowed)) ?? Data()
        return String(decoding: data, as: UTF8.self)
    }
}
