import AppKit
import SwiftUI

/// An HTTP method in monospaced text on a faint tint. The text carries the meaning; the tint only helps scanning.
struct MethodBadge: View {
    let method: String

    var body: some View {
        let text = Self.textColor(for: method)
        Text(method)
            .font(.caption.weight(.semibold).monospaced())
            .foregroundStyle(text)
            .frame(minWidth: 52)
            .padding(.vertical, 2)
            .background(text.opacity(0.15), in: RoundedRectangle(cornerRadius: 4))
            .accessibilityLabel("Method \(method)")
    }

    /// Text colors chosen for at least 4.5:1 against their tinted background in light and dark appearance.
    private static func textColor(for method: String) -> Color {
        switch method {
        case "GET": return adaptive(light: 0x14581F, dark: 0x7EE08E)
        case "POST": return adaptive(light: 0x084A9A, dark: 0x8AB8FF)
        case "PUT", "PATCH": return adaptive(light: 0x7E3B00, dark: 0xFFB86B)
        case "DELETE": return adaptive(light: 0x9E1F17, dark: 0xFF8A80)
        default: return .primary
        }
    }

    private static func adaptive(light: UInt32, dark: UInt32) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            return NSColor(hex: isDark ? dark : light)
        })
    }
}

private extension NSColor {
    convenience init(hex: UInt32) {
        self.init(
            srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
            green: CGFloat((hex >> 8) & 0xFF) / 255,
            blue: CGFloat(hex & 0xFF) / 255,
            alpha: 1
        )
    }
}
