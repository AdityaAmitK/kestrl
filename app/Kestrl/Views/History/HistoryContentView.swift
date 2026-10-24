import SwiftUI

/// Recorded events, newest first, with a full-width table. Select two rows to compare them.
struct HistoryContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        VStack(spacing: 0) {
            filterBar
            Divider()
            Table(store.events, selection: $store.selectedEventIDs) {
                TableColumn("Time") { event in
                    Text(TimeFormat.relative(ms: event.ts))
                        .help(TimeFormat.absolute(ms: event.ts))
                }
                .width(min: 70, ideal: 84)

                TableColumn("Status") { event in
                    if event.kind == "http" {
                        StatusBadge(status: event.status, error: nil)
                    } else {
                        Text("—")
                            .foregroundStyle(.secondary)
                    }
                }
                .width(min: 110, ideal: 130)

                TableColumn("Method") { event in
                    if let method = event.method {
                        MethodBadge(method: method)
                    }
                }
                .width(min: 64, ideal: 76)

                TableColumn("Summary") { event in
                    Text(event.summary)
                        .font(.body.monospaced())
                        .lineLimit(1)
                        .truncationMode(.tail)
                }
                .width(min: 200, ideal: 360)

                TableColumn("Actor") { event in
                    ActorLabel(actor: event.actor)
                }
                .width(min: 90, ideal: 110)

                TableColumn("Source") { event in
                    Text(event.source)
                        .foregroundStyle(.secondary)
                }
                .width(min: 60, ideal: 70)

                TableColumn("Tags") { event in
                    Text(event.tags.map { "#\($0)" }.joined(separator: " "))
                        .foregroundStyle(.secondary)
                }
                .width(min: 80, ideal: 120)

                TableColumn("Duration") { event in
                    if let duration = event.durationMs {
                        Text("\(duration) ms")
                            .monospacedDigit()
                    }
                }
                .width(min: 70, ideal: 84)
            }
            .overlay {
                if store.events.isEmpty {
                    ContentUnavailableView(
                        store.historyText.isEmpty && store.historyActor == nil ? "No history yet" : "No matching events",
                        systemImage: "clock.arrow.circlepath",
                        description: Text("Requests you send appear here.")
                    )
                }
            }
        }
        .task(id: store.historyQueryKey) {
            // Cheap enough to poll: keeps the list current while the section is visible.
            while !Task.isCancelled {
                await store.refreshEvents()
                try? await Task.sleep(for: .seconds(3))
            }
        }
        .onChange(of: store.selectedEventIDs) { _, _ in
            Task { await store.eventSelectionChanged() }
        }
    }

    /// Search, the actor filter, and Compare for two selected rows. They sit in the pane, not the toolbar, next to the rows they change.
    private var filterBar: some View {
        HStack(spacing: 12) {
            TextField("Search history", text: $store.historyText)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 320)
                .accessibilityLabel("Search history")

            actorMenu

            Spacer()

            if store.selectedEventIDs.count == 2 {
                Button {
                    Task { await store.compareSelectedEvents() }
                } label: {
                    Label("Compare", systemImage: "arrow.left.arrow.right")
                }
                .help("Compare the two selected events")
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private var actorMenu: some View {
        Menu {
            Picker("Actor", selection: $store.historyActor) {
                Text("Any actor").tag(String?.none)
                ForEach(store.knownActors, id: \.self) { actor in
                    Text(actor).tag(String?.some(actor))
                }
            }
            .pickerStyle(.inline)
        } label: {
            Label(store.historyActor ?? "Any actor", systemImage: "person.2")
        }
        .fixedSize()
        .help("Show events from one actor")
        .accessibilityLabel("Actor filter: \(store.historyActor ?? "any actor")")
    }
}
