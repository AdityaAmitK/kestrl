import SwiftUI

/// The live log of the selected service. The filter, Copy and Clear sit above the lines.
struct LogsDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        if let service = store.selectedLogService {
            VStack(spacing: 0) {
                controls(for: service)
                Divider()
                LogLinesView(store: store)
                Divider()
                footer(for: service)
            }
            .task(id: store.logStreamKey) {
                await store.followSelectedService()
            }
        } else {
            ContentUnavailableView(
                "Select a service",
                systemImage: "text.alignleft",
                description: Text("Its live log appears here.")
            )
        }
    }

    private func controls(for service: String) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Text(service)
                .font(.headline)
                .padding(.top, 4)

            VStack(alignment: .leading, spacing: 4) {
                TextField("Filter lines with a regular expression", text: $store.logFilterText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 320)
                    .accessibilityLabel("Filter lines")
                    .onChange(of: store.logFilterText) { _, _ in
                        store.logFilterTextChanged()
                    }
                if let error = store.logFilterError {
                    Text(error)
                        .font(.caption)
                        .foregroundStyle(Color(nsColor: .systemRed))
                }
            }

            Spacer()

            Button("Copy", systemImage: "doc.on.doc") {
                store.copySelectedLogLines()
            }
            .disabled(store.selectedLogSeqs.isEmpty)
            .help("Copy the selected lines")

            Button("Clear", systemImage: "eraser") {
                store.clearLogView()
            }
            .help("Clear the view. The service keeps running.")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private func footer(for service: String) -> some View {
        Text(store.logStatus ?? "Following \(service) · \(store.logLines.count) lines")
            .font(.caption)
            .foregroundStyle(.secondary)
            .monospacedDigit()
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 16)
            .padding(.vertical, 6)
    }
}
