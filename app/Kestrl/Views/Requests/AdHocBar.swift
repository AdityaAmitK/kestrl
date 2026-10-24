import SwiftUI

/// Quick requests without a file: method, URL and Send.
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
            .fixedSize()

            TextField("Ad-hoc URL, such as http://localhost:4000/health", text: $store.adHocURL)
                .textFieldStyle(.roundedBorder)
                .font(.body.monospaced())
                .onSubmit {
                    Task { await store.sendAdHocRequest() }
                }

            Button {
                Task { await store.sendAdHocRequest() }
            } label: {
                Label("Send ad-hoc request", systemImage: "paperplane")
            }
            .labelStyle(.iconOnly)
            .help("Send the ad-hoc request")
            .disabled(!store.canSendAdHoc)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(.bar)
    }
}
