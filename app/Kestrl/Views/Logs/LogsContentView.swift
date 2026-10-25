import SwiftUI

/// The services Kestrl runs or follows, with their ports and uptime. Refreshed every few seconds.
struct LogsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedLogService) {
            ForEach(store.services, id: \.name) { service in
                VStack(alignment: .leading, spacing: 2) {
                    Label(service.name, systemImage: "terminal")
                    Text(details(for: service))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                .tag(service.name)
            }
        }
        .listStyle(.plain)
        .environment(\.defaultMinListRowHeight, 26)
        .overlay {
            if store.services.isEmpty {
                EmptyPane(
                    "No services running",
                    systemImage: "text.alignleft",
                    description: Text("Start one with kestrl up api -- npm run dev. Its logs appear here.")
                )
            }
        }
        .task {
            while !Task.isCancelled {
                await store.loadServices()
                try? await Task.sleep(for: .seconds(3))
            }
        }
    }

    private func details(for service: ServiceInfo) -> String {
        let ports = service.ports.isEmpty ? "No port" : service.ports.map { ":\($0)" }.joined(separator: ", ")
        return "\(ports) · up \(TimeFormat.uptime(sinceMs: service.startedAtMs))"
    }
}
