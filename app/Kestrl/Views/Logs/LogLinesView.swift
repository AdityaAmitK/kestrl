import AppKit
import SwiftUI

/// The lines of the selected service in a lazy stack, so a long log stays smooth. Follows the newest line until the user scrolls up.
struct LogLinesView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(store.logLines, id: \.seq) { line in
                        LogLineRow(line: line, isSelected: store.selectedLogSeqs.contains(line.seq))
                            .id(line.seq)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                store.selectLogLine(line.seq, modifiers: NSEvent.modifierFlags)
                            }
                    }
                }
                .padding(.vertical, 6)
            }
            .background(Color(nsColor: .textBackgroundColor))
            .overlay(alignment: .bottomTrailing) {
                if !store.logsFollowing, let last = store.logLines.last {
                    Button {
                        store.logsFollowing = true
                        proxy.scrollTo(last.seq, anchor: .bottom)
                    } label: {
                        Label("Jump to latest", systemImage: "arrow.down.to.line")
                    }
                    .buttonStyle(.bordered)
                    .background(.bar, in: Capsule())
                    .padding(16)
                }
            }
            .onChange(of: store.logLines.last?.seq) { _, last in
                guard store.logsFollowing, let last else { return }
                proxy.scrollTo(last, anchor: .bottom)
            }
            // Moving up means the user scrolled. New lines grow the content but leave the offset alone, so they never pause the view.
            .onScrollGeometryChange(for: CGFloat.self, of: { $0.contentOffset.y }) { old, new in
                if new < old - 1 {
                    store.logsFollowing = false
                }
            }
            .onScrollGeometryChange(for: Bool.self, of: { geometry in
                geometry.contentOffset.y + geometry.containerSize.height >= geometry.contentSize.height - 24
            }) { _, atBottom in
                if atBottom {
                    store.logsFollowing = true
                }
            }
        }
        .overlay {
            if store.logLines.isEmpty {
                ContentUnavailableView(
                    store.logStatus == nil ? "Waiting for output" : "Not streaming",
                    systemImage: "text.alignleft",
                    description: Text(store.logStatus ?? "New lines appear here as the service writes them.")
                )
            }
        }
    }
}
