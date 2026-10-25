import SwiftUI

/// Editable unresolved request values. Sends never write files; Save is explicit.
struct RequestPane: View {
    @Bindable var store: WorkspaceStore
    @State private var part: Part = .params

    private enum Part: String, CaseIterable, Identifiable {
        case params = "Params"
        case headers = "Headers"
        case body = "Body"
        case options = "Options"
        var id: Self { self }
    }

    var body: some View {
        content
            // Re-chosen once a request's file has loaded, not while it is being edited.
            .task(id: "\(store.selectedRequest ?? "<new>")|\(store.requestDetail?.method ?? "")|\(store.requestDetail?.url ?? "")") {
                part = preferredPart()
            }
    }

    private var content: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Picker("Request editor", selection: $part) {
                    ForEach(Part.allCases) { Text(title(for: $0)).tag($0) }
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
                table($store.requestDraft.query, separator: "=", key: "parameter")
            case .headers:
                table($store.requestDraft.headers, separator: ":", key: "header")
            case .body:
                if store.requestDraft.bodyKind == "none" {
                    EmptyPane("No request body", systemImage: "doc", description: Text("Choose JSON, text or form above to include a body."))
                } else if store.requestDraft.bodyKind == "form" {
                    table($store.requestDraft.body, separator: "=", key: "field")
                } else {
                    editor($store.requestDraft.body, label: "Request body", hint: "Request body · {{variables}} and {{secret:name}} are resolved when sent")
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

    private func table(_ value: Binding<String>, separator: Character, key: String) -> some View {
        KeyValueLinesEditor(
            text: value,
            separator: separator,
            keyPlaceholder: key,
            valuePlaceholder: "value",
            variables: store.editorVariables,
            environmentName: store.selectedEnvironment
        )
        .background(Color(nsColor: .textBackgroundColor))
    }

    /// Tab titles carry counts, so a request's params and headers are visible without opening each tab.
    private func title(for part: Part) -> String {
        switch part {
        case .params: return count(store.requestDraft.query).map { "Params \($0)" } ?? "Params"
        case .headers: return count(store.requestDraft.headers).map { "Headers \($0)" } ?? "Headers"
        case .body: return store.requestDraft.bodyKind == "none" ? "Body" : "Body ●"
        case .options: return "Options"
        }
    }

    private func count(_ lines: String) -> Int? {
        let total = lines.split(separator: "\n").filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }.count
        return total > 0 ? total : nil
    }

    /// Opens the tab that has content: the body when there is one, else params, else headers.
    private func preferredPart() -> Part {
        if store.requestDraft.bodyKind != "none" { return .body }
        if count(store.requestDraft.query) != nil { return .params }
        if count(store.requestDraft.headers) != nil { return .headers }
        return .params
    }

    private func editor(_ value: Binding<String>, label: String, hint: String) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(hint).font(.system(size: 11)).foregroundStyle(.secondary)
                .padding(.horizontal, 12).padding(.vertical, 6)
            VariableTextView(text: value, variables: store.editorVariables, label: label, environmentName: store.selectedEnvironment)
                .padding(.horizontal, 8).accessibilityLabel(label)
        }
        .background(Color(nsColor: .textBackgroundColor))
    }
}
