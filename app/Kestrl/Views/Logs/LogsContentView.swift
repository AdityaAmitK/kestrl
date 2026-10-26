import AppKit
import SwiftUI

/// The services Kestrl runs or follows, with their ports and uptime, and the ports shared through
/// Cloudflare tunnels. Refreshed every few seconds.
struct LogsContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.selectedLogService) {
            Section("Services") {
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
            if !store.shares.isEmpty {
                Section("Shares") {
                    ForEach(store.shares, id: \.id) { share in
                        ShareRow(share: share, store: store)
                    }
                }
            }
        }
        .listStyle(.plain)
        .environment(\.defaultMinListRowHeight, 26)
        .overlay {
            if store.services.isEmpty && store.shares.isEmpty {
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

/// One shared port: its URL, who can reach it and when it closes. Not selectable; actions are in
/// the context menu.
private struct ShareRow: View {
    let share: ShareInfo
    let store: WorkspaceStore

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Label(share.url ?? "Starting…", systemImage: share.isPublic ? "globe" : "lock.shield")
                .lineLimit(1)
                .truncationMode(.middle)
            Text(details)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .help(share.isPublic ? "Anyone with the URL can reach this port" : "Protected by Cloudflare Access")
        .accessibilityElement(children: .combine)
        .contextMenu {
            if let url = share.url {
                Button("Copy URL") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(url, forType: .string)
                }
                if let link = URL(string: url) {
                    Button("Open in Browser") { NSWorkspace.shared.open(link) }
                }
                Divider()
            }
            Button("Close Share", role: .destructive) {
                Task { await store.closeShare(id: share.id) }
            }
        }
    }

    private var details: String {
        let access = share.isPublic ? "Public" : "Protected"
        return ":\(share.port) · \(access) · closes in \(TimeFormat.remaining(untilMs: share.expiresAtMs))"
    }
}
