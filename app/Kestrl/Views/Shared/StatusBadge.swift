import AppKit
import SwiftUI

/// The outcome of a send: the status code with its reason phrase, as text, with a symbol and tint for quick scanning. Color never carries the meaning alone.
struct StatusBadge: View {
    let status: UInt16?
    let error: String?

    var body: some View {
        let style = Style(status: status, error: error)
        HStack(spacing: 6) {
            Image(systemName: style.symbol)
                .foregroundStyle(style.tint)
            Text(style.text)
                .foregroundStyle(.primary)
        }
        .font(.system(size: 11, weight: .medium))
        .padding(.horizontal, 8)
        .padding(.vertical, 3)
        .background(style.tint.opacity(0.16), in: RoundedRectangle(cornerRadius: 4))
        .accessibilityElement(children: .combine)
    }

    private struct Style {
        let text: String
        let symbol: String
        let tint: Color

        init(status: UInt16?, error: String?) {
            guard let status else {
                text = error ?? "Failed"
                symbol = "xmark.octagon.fill"
                tint = Color(nsColor: .systemRed)
                return
            }
            let phrase = HTTPStatusText.phrase(for: status).map { " \($0)" } ?? ""
            text = "\(status)\(phrase)"
            switch status {
            case 200..<300:
                symbol = "checkmark.circle.fill"
                tint = Color(nsColor: .systemGreen)
            case 300..<400:
                symbol = "arrow.turn.up.right"
                tint = Color(nsColor: .systemBlue)
            case 400..<500:
                symbol = "exclamationmark.circle.fill"
                tint = Color(nsColor: .systemOrange)
            case 500..<600:
                symbol = "xmark.octagon.fill"
                tint = Color(nsColor: .systemRed)
            default:
                symbol = "circle.fill"
                tint = .secondary
            }
        }
    }
}
