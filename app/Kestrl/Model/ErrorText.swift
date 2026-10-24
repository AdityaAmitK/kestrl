import Foundation

/// The message to show for an error. FFI errors already carry readable text.
enum ErrorText {
    static func message(for error: Error) -> String {
        guard let kestrlError = error as? KestrlError else {
            return error.localizedDescription
        }
        switch kestrlError {
        case .File(let message),
             .Request(let message),
             .Secret(let message),
             .History(let message),
             .InvalidInput(let message):
            return message
        }
    }
}
