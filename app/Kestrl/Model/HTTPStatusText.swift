/// Reason phrases for the status codes an API commonly returns.
enum HTTPStatusText {
    private static let phrases: [UInt16: String] = [
        200: "OK", 201: "Created", 202: "Accepted", 204: "No Content",
        206: "Partial Content",
        301: "Moved Permanently", 302: "Found", 303: "See Other", 304: "Not Modified",
        307: "Temporary Redirect", 308: "Permanent Redirect",
        400: "Bad Request", 401: "Unauthorized", 403: "Forbidden", 404: "Not Found",
        405: "Method Not Allowed", 406: "Not Acceptable", 408: "Request Timeout",
        409: "Conflict", 410: "Gone", 415: "Unsupported Media Type",
        422: "Unprocessable Content", 429: "Too Many Requests",
        500: "Internal Server Error", 501: "Not Implemented", 502: "Bad Gateway",
        503: "Service Unavailable", 504: "Gateway Timeout",
    ]

    static func phrase(for code: UInt16) -> String? {
        phrases[code]
    }
}
