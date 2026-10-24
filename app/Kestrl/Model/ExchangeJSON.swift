import Foundation

/// The full response as JSON for "Copy JSON": status, timing, headers, body and the names of masked secrets.
enum ExchangeJSON {
    static func make(
        status: UInt16?,
        durationMs: UInt64,
        headers: [KeyValue],
        body: String,
        redacted: [String],
        error: String?
    ) -> String {
        let parsedBody: Any = (try? JSONSerialization.jsonObject(with: Data(body.utf8), options: .fragmentsAllowed)) ?? body
        let object: [String: Any] = [
            "status": status.map { Int($0) as Any } ?? NSNull(),
            "ms": Int(clamping: durationMs),
            "headers": Dictionary(headers.map { ($0.key, $0.value) }, uniquingKeysWith: { _, last in last }),
            "body": parsedBody,
            "redacted": redacted,
            "error": error.map { $0 as Any } ?? NSNull(),
        ]
        guard
            let data = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]),
            let text = String(data: data, encoding: .utf8)
        else {
            return "{}"
        }
        return text
    }
}
