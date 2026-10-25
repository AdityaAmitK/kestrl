import SwiftUI

/// The open request as its file defines it, with variables unresolved: params, headers or body.
struct RequestPane: View {
    let store: WorkspaceStore
    let detail: RequestDetail

    @State private var part: Part = .params

    private enum Part: String, CaseIterable, Identifiable {
        case params = "Params"
        case headers = "Headers"
        case body = "Body"

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
            .padding(.vertical, 8)

            Divider()

            switch part {
            case .params:
                keyValues(detail.query, empty: "This request has no params.")
            case .headers:
                keyValues(detail.headers, empty: "This request has no headers.")
            case .body:
                bodyContent
            }
        }
    }

    @ViewBuilder private func keyValues(_ pairs: [KeyValue], empty: String) -> some View {
        if pairs.isEmpty {
            EmptyPane("Nothing here", systemImage: "list.bullet", description: Text(empty))
        } else {
            ScrollView {
                KeyValueGrid(title: part.rawValue, pairs: pairs)
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    @ViewBuilder private var bodyContent: some View {
        if let body = store.requestBody {
            JSONTextView(text: body.text)
        } else {
            EmptyPane("No body", systemImage: "doc", description: Text("This request sends no body."))
        }
    }
}
