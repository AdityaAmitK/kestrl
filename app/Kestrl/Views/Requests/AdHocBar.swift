import SwiftUI

/// The editable bar for a request without a file: method, URL and Send. Opened by File ▸ New Request.
struct AdHocBar: View {
    @Bindable var store: WorkspaceStore

    private static let methods = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]

    var body: some View {
        HStack(spacing: 12) {
            Picker("Method", selection: $store.adHocMethod) {
                ForEach(Self.methods, id: \.self) { method in
                    Text(method).font(.body.monospaced()).tag(method)
                }
            }
            .labelsHidden()
            .fixedSize()

            TextField("URL, such as http://localhost:4000/health", text: $store.adHocURL)
                .textFieldStyle(.roundedBorder)
                .font(.body.monospaced())
                .onSubmit {
                    Task { await store.sendAdHocRequest() }
                }

            SendButton(store: store)
        }
    }
}
