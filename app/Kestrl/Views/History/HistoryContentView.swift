import SwiftUI

/// Recorded events, newest first. Multi-select two rows to compare them.
struct HistoryContentView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        Table(store.events, selection: $store.selectedEventIDs) {
            TableColumn("Time") { event in
                Text(TimeFormat.relative(ms: event.ts))
                    .help(TimeFormat.absolute(ms: event.ts))
            }
            .width(min: 70, ideal: 84)

            TableColumn("Actor") { event in
                ActorLabel(actor: event.actor)
            }
            .width(min: 90, ideal: 110)

            TableColumn("Source") { event in
                Text(event.source)
                    .foregroundStyle(.secondary)
            }
            .width(min: 60, ideal: 70)

            TableColumn("Summary") { event in
                Text(event.summary)
                    .font(.body.monospaced())
                    .lineLimit(1)
            }

            TableColumn("Tags") { event in
                Text(event.tags.map { "#\($0)" }.joined(separator: " "))
                    .foregroundStyle(.secondary)
            }
            .width(min: 80, ideal: 120)
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
        .searchable(text: $store.historyText, placement: .toolbar, prompt: "Search history")
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
        .toolbar {
            ToolbarItem {
                actorMenu
            }
            if store.selectedEventIDs.count == 2 {
                ToolbarItem {
                    Button {
                        Task { await store.compareSelectedEvents() }
                    } label: {
                        Label("Compare", systemImage: "arrow.left.arrow.right")
                    }
                    .help("Compare the two selected events")
                }
            }
        }
        .navigationTitle("History")
        .navigationSplitViewColumnWidth(min: 300, ideal: 420)
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
        .help("Show events from one actor")
    }
}
