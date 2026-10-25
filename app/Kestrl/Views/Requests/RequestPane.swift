import SwiftUI

/// Editable unresolved request values. Sends never write files; Save is explicit.
struct RequestPane: View {
    @Bindable var store: WorkspaceStore
    @State private var part: Part = .body

    private enum Part: String, CaseIterable, Identifiable {
        case params = "Params"
        case headers = "Headers"
        case body = "Body"
        case options = "Options"
        var id: Self { self }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Picker("Request editor", selection: $part) {
                    ForEach(Part.allCases) { Text($0.rawValue).tag($0) }
                }
                .pickerStyle(.segmented).labelsHidden().fixedSize()
                Spacer()
                if part == .body {
                    Picker("Body type", selection: $store.requestDraft.bodyKind) {
                        Text("No body").tag("none")
                        Text("JSON").tag("json")
                        Text("Text").tag("text")
                        Text("Form").tag("form")
                    }
                    .labelsHidden().frame(width: 100)
                    .help("Request body type")
                }
            }
            .controlSize(.small).padding(.horizontal, 12).padding(.vertical, 7)
            Divider()
            switch part {
            case .params:
                editor($store.requestDraft.query, label: "Query parameters", hint: "name=value · one parameter per line")
            case .headers:
                editor($store.requestDraft.headers, label: "Request headers", hint: "Content-Type: application/json · one header per line")
            case .body:
                if store.requestDraft.bodyKind == "none" {
                    EmptyPane("No request body", systemImage: "doc", description: Text("Choose JSON, text or form above to include a body."))
                } else {
                    editor($store.requestDraft.body, label: "Request body", hint: store.requestDraft.bodyKind == "form" ? "name=value · one field per line" : "Request body · {{variables}} and {{secret:name}} are resolved when sent")
                }
            case .options:
                Form {
                    TextField("Name", text: $store.requestDraft.name)
                    TextField("Profiles", text: $store.requestDraft.profiles, prompt: Text("admin, debug"))
                    Text("Use {{secret:name}} for credentials. Drafts stay in memory until you save a request file.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                .formStyle(.grouped)
            }
        }
    }

    private func editor(_ value: Binding<String>, label: String, hint: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(hint).font(.system(size: 11)).foregroundStyle(.secondary)
                .padding(.horizontal, 12).padding(.vertical, 6)
            TextEditor(text: value).font(.system(size: 12, design: .monospaced))
                .padding(.horizontal, 8).accessibilityLabel(label)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}
