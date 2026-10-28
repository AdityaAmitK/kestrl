import Foundation

/// Parses a pasted command as data; never runs a shell or reads referenced files.
enum CurlImport {
    static func parse(_ command: String) throws -> RequestEditorDraft {
        let words = try tokenize(command)
        guard words.first == "curl" else { throw invalid("Paste a command beginning with curl.") }
        var draft = RequestEditorDraft()
        var method: String?
        var headers: [String] = []
        var bodies: [String] = []
        var index = 1
        while index < words.count {
            let word = words[index]
            index += 1
            let option: String
            var attached: String?
            if word.hasPrefix("--"), let equals = word.firstIndex(of: "=") {
                option = String(word[..<equals])
                attached = String(word[word.index(after: equals)...])
            } else if word.count > 2, ["-X", "-H", "-d", "-b"].contains(String(word.prefix(2))) {
                option = String(word.prefix(2))
                attached = String(word.dropFirst(2))
            } else {
                option = word
            }
            func value() throws -> String {
                if let attached { return attached }
                guard index < words.count else { throw invalid("A cURL option is missing its value.") }
                defer { index += 1 }
                return words[index]
            }
            switch option {
            case "-X", "--request":
                method = try value().uppercased()
            case "--url":
                guard draft.url.isEmpty else { throw invalid("Import one URL at a time.") }
                draft.url = try value()
            case "-H", "--header":
                let header = try value()
                guard header.contains(":"), !header.contains(where: { $0.isNewline }) else {
                    throw invalid("Only inline name: value headers are supported.")
                }
                headers.append(header)
            case "-b", "--cookie":
                let cookie = try value()
                guard cookie.contains("="), !cookie.contains(where: { $0.isNewline }) else {
                    throw invalid("Cookie files are not supported; paste an inline cookie.")
                }
                headers.append("Cookie: " + cookie)
            case "-d", "--data", "--data-ascii", "--data-raw", "--data-binary", "--json":
                let body = try value()
                guard option == "--data-raw" || !body.hasPrefix("@") else {
                    throw invalid("File bodies are not supported; paste the body inline.")
                }
                bodies.append(body)
                if option == "--json" {
                    headers.append("Content-Type: application/json")
                    headers.append("Accept: application/json")
                }
            case "-I", "--head":
                method = "HEAD"
            case "--compressed", "-L", "--location":
                break // The shared HTTP client handles decompression and redirects.
            default:
                guard !word.hasPrefix("-") else {
                    throw invalid("This command contains an unsupported cURL option. Use URL, request, header, cookie or inline data options.")
                }
                guard draft.url.isEmpty else { throw invalid("Import one URL at a time.") }
                draft.url = word
            }
        }
        guard draft.url.hasPrefix("http://") || draft.url.hasPrefix("https://") else {
            throw invalid("The cURL command needs an HTTP or HTTPS URL.")
        }
        draft.method = method ?? (bodies.isEmpty ? "GET" : "POST")
        guard !draft.method.isEmpty, draft.method.utf8.allSatisfy({ (65...90).contains($0) || $0 == 45 }) else {
            throw invalid("The request method is invalid.")
        }
        if !bodies.isEmpty {
            draft.body = bodies.joined(separator: "&")
            if !headers.contains(where: { $0.lowercased().hasPrefix("content-type:") }) {
                headers.append("Content-Type: application/x-www-form-urlencoded")
            }
            draft.bodyKind = headers.contains(where: {
                $0.lowercased().hasPrefix("content-type:") && $0.lowercased().contains("json")
            }) ? "json" : "text"
        }
        draft.headers = headers.joined(separator: "\n")
        return draft
    }

    private static func invalid(_ message: String) -> KestrlError {
        .InvalidInput(message: message)
    }

    /// POSIX-style quotes and line continuations, without expansion or execution.
    private static func tokenize(_ command: String) throws -> [String] {
        let chars = Array(command)
        var words: [String] = []
        var word = ""
        var quote: Character?
        var started = false
        var index = 0
        while index < chars.count {
            let char = chars[index]
            index += 1
            if char == "\\", quote != "'" {
                guard index < chars.count else { throw invalid("The command ends with an incomplete escape.") }
                let next = chars[index]
                if next == "\n" { index += 1; continue }
                if quote == "\"", next != "\"", next != "\\", next != "$", next != "`" {
                    word.append(char)
                } else {
                    word.append(next)
                    index += 1
                }
                started = true
            } else if let current = quote {
                if current == "\"", char == "$" || char == "`" {
                    throw invalid("Shell expansions are not supported. Paste literal values in single quotes.")
                }
                if char == current { quote = nil } else { word.append(char) }
            } else if char == "'" || char == "\"" {
                quote = char
                started = true
            } else if char.isWhitespace {
                if started { words.append(word); word = ""; started = false }
            } else {
                guard ![";", "|", "&", "<", ">", "`", "$"].contains(char) else {
                    throw invalid("Shell operators and expansions are not supported. Paste a literal cURL command.")
                }
                word.append(char)
                started = true
            }
        }
        guard quote == nil else { throw invalid("The cURL command has an unclosed quote.") }
        if started { words.append(word) }
        return words
    }
}
