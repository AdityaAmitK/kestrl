import AppKit
import SwiftUI

/// One log line: a local time gutter, a stderr marker, and the text. stderr is marked by its label and a tint, so color alone never carries it.
/// A JSON line with `level` and `message` shows a level chip, the message, then its other keys dimmed. Hovering shows the raw line.
struct LogLineRow: View {
    let line: LogLine
    let isSelected: Bool
    /// Off, the line stays on one row and the log scrolls sideways
    let wrapsLines: Bool

    var body: some View {
        let isError = line.stream == "stderr"
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text(TimeFormat.clock(ms: line.tsMs))
                .foregroundStyle(.secondary)
                .monospacedDigit()
                .lineLimit(1)
                .fixedSize()
                .frame(width: 110, alignment: .leading)
            if isError {
                Text("stderr")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, 5)
                    .padding(.vertical, 1)
                    .background(Color(nsColor: .systemRed).opacity(0.2), in: RoundedRectangle(cornerRadius: 4))
            }
            content
        }
        .font(.body.monospaced())
        .padding(.horizontal, 12)
        .padding(.vertical, 1)
        .background(rowFill(isError: isError))
        .help(line.text)
        .accessibilityElement(children: .combine)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    @ViewBuilder private var content: some View {
        if let json = JSONLogLine(text: line.text) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                levelChip(json.level)
                Text(json.message)
                    .fixedSize(horizontal: !wrapsLines, vertical: false)
                if !json.fields.isEmpty {
                    Text(json.fields.map { "\($0.key)=\($0.value)" }.joined(separator: " "))
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: !wrapsLines, vertical: false)
                }
            }
            .frame(maxWidth: wrapsLines ? .infinity : nil, alignment: .leading)
        } else {
            Text(line.text)
                .fixedSize(horizontal: !wrapsLines, vertical: false)
                .frame(maxWidth: wrapsLines ? .infinity : nil, alignment: .leading)
        }
    }

    private func levelChip(_ level: String) -> some View {
        let tint = levelTint(level)
        return Text(level)
            .font(.caption.weight(.semibold))
            .foregroundStyle(tint)
            .padding(.horizontal, 5)
            .padding(.vertical, 1)
            .background(tint.opacity(0.15), in: RoundedRectangle(cornerRadius: 4))
            .fixedSize()
    }

    private func levelTint(_ level: String) -> Color {
        switch level.lowercased() {
        case "error", "fatal": return Color(nsColor: .systemRed)
        case "warn", "warning": return Color(nsColor: .systemOrange)
        case "info": return Color(nsColor: .systemBlue)
        default: return Color(nsColor: .secondaryLabelColor)
        }
    }

    private func rowFill(isError: Bool) -> Color {
        if isSelected { return Color.accentColor.opacity(0.3) }
        if isError { return Color(nsColor: .systemRed).opacity(0.08) }
        return Color.clear
    }
}
