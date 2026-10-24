import SwiftUI

/// The selected event, or the compare prompt when two are selected.
struct HistoryDetailView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        switch store.selectedEventIDs.count {
        case 0:
            ContentUnavailableView(
                "No event selected",
                systemImage: "clock.arrow.circlepath",
                description: Text("Select an event to see its request and response.")
            )
        case 1:
            if let shown = store.eventDetail {
                EventDetailView(store: store, shown: shown)
            } else {
                ProgressView()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        case let count:
            ContentUnavailableView {
                Label("\(count) events selected", systemImage: "square.on.square")
            } description: {
                Text("Select exactly two events to compare them.")
            } actions: {
                if count == 2 {
                    Button("Compare") {
                        Task { await store.compareSelectedEvents() }
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
        }
    }
}
