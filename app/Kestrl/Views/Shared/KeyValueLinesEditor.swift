import SwiftUI

/// A key/value table over the draft's `name<separator>value` lines (one pair per line), so
/// query params and headers edit like a table while the draft format stays plain text.
struct KeyValueLinesEditor: View {
    @Binding var text: String
    /// "=" for query params and form fields, ":" for headers.
    let separator: Character
    let keyPlaceholder: String
    let valuePlaceholder: String
    let variables: [VariableInfo]
    let environmentName: String?

    @State private var rows: [Row] = []

    private struct Row: Identifiable, Equatable {
        let id = UUID()
        var key: String
        var value: String
        static func == (lhs: Row, rhs: Row) -> Bool { lhs.key == rhs.key && lhs.value == rhs.value }
    }

    var body: some View {
        ScrollView {
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 8) {
                GridRow {
                    Text(keyPlaceholder.capitalized)
                        .frame(minWidth: 140, idealWidth: 200, maxWidth: 260, alignment: .leading)
                    Text("Value")
                        .frame(minWidth: 200, maxWidth: .infinity, alignment: .leading)
                    Color.clear.frame(width: 20, height: 1)
                }
                .font(.caption.weight(.medium))
                .foregroundStyle(.secondary)
                ForEach($rows) { $row in
                    GridRow(alignment: .top) {
                        TextField(keyPlaceholder, text: $row.key)
                            .textFieldStyle(.roundedBorder)
                            .font(.body.monospaced())
                            .frame(minWidth: 140, idealWidth: 200, maxWidth: 260, minHeight: 24, maxHeight: 24)
                            .accessibilityLabel(keyPlaceholder.capitalized)
                        VariableTextView(
                            text: $row.value,
                            variables: variables,
                            label: "Value of \(row.key.isEmpty ? keyPlaceholder : row.key)",
                            environmentName: environmentName,
                            singleLine: true
                        )
                        .frame(minWidth: 200, maxWidth: .infinity, minHeight: 24, maxHeight: 24)
                        Button {
                            rows.removeAll { $0.id == row.id }
                        } label: {
                            Image(systemName: "minus.circle")
                        }
                        .buttonStyle(.borderless)
                        .frame(minHeight: 24)
                        .help("Remove")
                        .accessibilityLabel("Remove \(row.key.isEmpty ? keyPlaceholder : row.key)")
                    }
                }
                GridRow {
                    Button {
                        rows.append(Row(key: "", value: ""))
                    } label: {
                        Label("Add \(keyPlaceholder)", systemImage: "plus")
                    }
                    .buttonStyle(.borderless)
                    .gridCellColumns(3)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(12)
        }
        .onAppear { rows = Self.parse(text, separator: separator) }
        .onChange(of: text) { _, newText in
            // External changes (another request selected, revert) replace the rows.
            if newText != Self.format(rows, separator: separator) {
                rows = Self.parse(newText, separator: separator)
            }
        }
        .onChange(of: rows) { _, newRows in
            let formatted = Self.format(newRows, separator: separator)
            if formatted != text { text = formatted }
        }
    }

    private static func parse(_ text: String, separator: Character) -> [Row] {
        text.split(separator: "\n", omittingEmptySubsequences: true).map { line in
            guard let index = line.firstIndex(of: separator) else {
                return Row(key: String(line).trimmingCharacters(in: .whitespaces), value: "")
            }
            return Row(
                key: String(line[..<index]).trimmingCharacters(in: .whitespaces),
                value: String(line[line.index(after: index)...]).trimmingCharacters(in: .whitespaces)
            )
        }
    }

    /// Rows without a key are dropped, so a half-added row never sends an empty param.
    private static func format(_ rows: [Row], separator: Character) -> String {
        rows.filter { !$0.key.trimmingCharacters(in: .whitespaces).isEmpty }
            .map { separator == ":" ? "\($0.key): \($0.value)" : "\($0.key)=\($0.value)" }
            .joined(separator: "\n")
    }
}
