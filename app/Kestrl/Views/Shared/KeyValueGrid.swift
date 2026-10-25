import SwiftUI

/// A titled list of name and value pairs, such as headers or query parameters. Renders nothing when empty.
struct KeyValueGrid: View {
    let title: String
    let pairs: [KeyValue]

    var body: some View {
        if !pairs.isEmpty {
            VStack(alignment: .leading, spacing: 6) {
                Text(title)
                    .font(.caption.weight(.semibold))
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 10) {
                    ForEach(pairs) { pair in
                        GridRow {
                            Text(pair.key)
                                .foregroundStyle(.secondary)
                                .gridColumnAlignment(.leading)
                            Text(pair.value)
                                .font(.body.monospaced())
                                .textSelection(.enabled)
                        }
                    }
                }
            }
        }
    }
}
