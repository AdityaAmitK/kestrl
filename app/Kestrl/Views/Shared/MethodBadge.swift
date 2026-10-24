import SwiftUI

/// An HTTP method in a quiet monospaced capsule.
struct MethodBadge: View {
    let method: String

    var body: some View {
        Text(method)
            .font(.caption.weight(.semibold).monospaced())
            .foregroundStyle(.primary)
            .frame(minWidth: 52)
            .padding(.vertical, 2)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
            .accessibilityLabel("Method \(method)")
    }
}
