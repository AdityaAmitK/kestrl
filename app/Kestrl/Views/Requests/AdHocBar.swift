import SwiftUI

/// The editable bar for a request without a file: method, URL and Send. Opened by File ▸ New Request.
struct AdHocBar: View {
    @Bindable var store: WorkspaceStore

    private static let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]

    var body: some View {
        HStack(spacing: 8) {
            Picker("Method", selection: $store.adHocMethod) {
                ForEach(Self.methods, id: \.self) { method in
                    Text(method).font(.body.monospaced()).tag(method)
                }
            }
            .labelsHidden()
            .frame(width: 88)

            VariableTextView(text: $store.adHocURL, variables: store.editorVariables, label: "Request URL", environmentName: store.selectedEnvironment, singleLine: true, onPaste: { store.pasteCurl($0) }) {
                Task { await store.sendAdHocRequest() }
            }
            .frame(height: 28)
            .help("Enter a URL or paste a cURL command")

            SendButton(store: store)
        }
    }
}
