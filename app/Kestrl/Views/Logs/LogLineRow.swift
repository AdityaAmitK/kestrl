import AppKit
import SwiftUI

/// One log line: a local time gutter, a stderr marker, and the text. stderr is marked by its label and a tint, so color alone never carries it.
struct LogLineRow: View {
    let line: LogLine
    let isSelected: Bool

    var body: some View {
        let isError = line.stream == "stderr"
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(TimeFormat.clock(ms: line.tsMs))
                .foregroundStyle(.secondary)
                .monospacedDigit()
                .fixedSize()
                .frame(width: 110, alignment: .leading)
            if isError {
                Text("stderr")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 5)
                    .padding(.vertical, 1)
                    .background(Color(nsColor: .systemRed).opacity(0.2), in: RoundedRectangle(cornerRadius: 4))
            }
            Text(line.text)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .font(.body.monospaced())
        .padding(.horizontal, 12)
        .padding(.vertical, 1)
        .background(rowFill(isError: isError))
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private func rowFill(isError: Bool) -> Color {
        if isSelected { return Color.accentColor.opacity(0.3) }
        if isError { return Color(nsColor: .systemRed).opacity(0.08) }
        return Color.clear
    }
}
