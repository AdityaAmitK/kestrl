import SwiftUI

/// Whether the last query on this connection could write. The text names the environment, and the symbol carries the meaning too.
struct WriteAccessBadge: View {
    /// nil until a query has run, because the core reports access only with a result
    let isReadOnly: Bool?
    let environment: String?

    var body: some View {
        if let isReadOnly {
            let place = environment.map { " in \($0)" } ?? ""
            Label(
                isReadOnly ? "Read-only\(place)" : "Writes allowed\(place)",
                systemImage: isReadOnly ? "lock.fill" : "pencil"
            )
            .font(.caption.weight(.semibold))
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(tint(isReadOnly).opacity(0.16), in: Capsule())
        } else {
            Label("Access shown after a query runs", systemImage: "questionmark.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
    }

    private func tint(_ isReadOnly: Bool) -> Color {
        isReadOnly ? Color(nsColor: .systemBlue) : Color(nsColor: .systemOrange)
    }
}
