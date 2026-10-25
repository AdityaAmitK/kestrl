import Foundation

/// A captured exchange as JSON for "Copy JSON": method, URL, status, timing, and both sides of the exchange. Secrets are already masked by the core.
enum CaptureJSON {
    static func make(_ exchange: CapturedExchange) -> String {
        let object: [String: Any] = [
            "method": exchange.method,
            "url": exchange.url,
            "status": exchange.status.map { Int($0) as Any } ?? NSNull(),
            "ms": Int(clamping: exchange.durationMs),
            "request": [
                "headers": headers(exchange.requestHeaders),
                "body": body(exchange.requestBody),
            ],
            "response": [
                "headers": headers(exchange.responseHeaders),
                "body": body(exchange.responseBody),
            ],
        ]
        guard
            let data = try? JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]),
            let text = String(data: data, encoding: .utf8)
        else {
            return "{}"
        }
        return text
    }

    /// A header that appears more than once, such as Set-Cookie, becomes an array so no value is dropped.
    private static func headers(_ pairs: [KeyValue]) -> [String: Any] {
        Dictionary(grouping: pairs, by: \.key).mapValues { group in
            group.count == 1 ? group[0].value as Any : group.map(\.value) as Any
        }
    }

    /// JSON bodies are embedded as JSON, anything else as a string.
    private static func body(_ text: String?) -> Any {
        guard let text else { return NSNull() }
        return (try? JSONSerialization.jsonObject(with: Data(text.utf8), options: .fragmentsAllowed)) ?? text
    }
}
