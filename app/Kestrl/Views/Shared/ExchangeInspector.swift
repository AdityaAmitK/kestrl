import SwiftUI

/// The body and headers of one side of an exchange, behind a Body | Headers switch.
struct ExchangeInspector: View {
    let headers: [KeyValue]
    let payload: PreparedBody?
    let emptyMessage: String

    @State private var part: Part = .body

    private enum Part: String, CaseIterable, Identifiable {
        case body = "Body"
        case headers = "Headers"

        var id: Self { self }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Picker("Show", selection: $part) {
                ForEach(Part.allCases) { option in
                    Text(option.rawValue).tag(option)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .controlSize(.small)
            .padding(.horizontal, 12)
            .padding(.vertical, 7)

            Divider()

            switch part {
            case .body:
                bodyContent
            case .headers:
                headersContent
            }
        }
    }

    @ViewBuilder private var bodyContent: some View {
        if let payload, !payload.text.isEmpty {
            JSONTextView(text: payload.text)
        } else {
            EmptyPane("No body", systemImage: "doc", description: Text(emptyMessage))
        }
    }

    @ViewBuilder private var headersContent: some View {
        if headers.isEmpty {
            EmptyPane("No headers", systemImage: "list.bullet")
        } else {
            ScrollView {
                KeyValueGrid(title: "Headers", pairs: headers)
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }
}
