import Foundation

/// A body ready to show. JSON is pretty-printed; anything else is kept as received.
struct PreparedBody: Sendable {
    let text: String
    let isJSON: Bool

    /// Parses and formats off the main actor, so a large response does not stall the UI.
    static func prepare(_ raw: String) async -> PreparedBody {
        await Task.detached(priority: .userInitiated) {
            guard
                let object = try? JSONSerialization.jsonObject(with: Data(raw.utf8), options: .fragmentsAllowed),
                let data = try? JSONSerialization.data(
                    withJSONObject: object,
                    options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes, .fragmentsAllowed]
                ),
                let text = String(data: data, encoding: .utf8)
            else {
                return PreparedBody(text: raw, isJSON: false)
            }
            return PreparedBody(text: text, isJSON: true)
        }.value
    }
}
