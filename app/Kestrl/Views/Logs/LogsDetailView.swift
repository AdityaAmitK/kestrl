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
            EmptyPane(
                "Select a service",
                systemImage: "text.alignleft",
                description: Text("Its live log appears here.")
            )
        }
    }

    private func controls(for service: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Text(service)
                .font(.headline)
                .padding(.top, 4)

            VStack(alignment: .leading, spacing: 4) {
                TextField("Filter lines (regex)", text: $store.logFilterText)
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

            Toggle(isOn: $store.wrapLogLines) {
                Image(systemName: "text.word.spacing")
            }
            .toggleStyle(.button)
            .help("Wrap log lines (⌥⌘W)")
            .accessibilityLabel("Wrap log lines")

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
        .controlSize(.small)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
    }

    private func footer(for service: String) -> some View {
        Text(store.logStatus ?? "\(store.logsFollowing ? "Following" : "Paused") \(service) · \(store.logLines.count) \(store.logLines.count == 1 ? "line" : "lines")")
            .font(.caption)
            .foregroundStyle(.secondary)
            .monospacedDigit()
            .frame(maxWidth: .infinity, alignment: .leading)
            .controlSize(.small)
        .padding(.horizontal, 12)
            .padding(.vertical, 6)
    }
}
