import SwiftUI

/// The section list. Each section's items appear in the content column.
struct SidebarView: View {
    @Bindable var store: WorkspaceStore

    var body: some View {
        List(selection: $store.section) {
            ForEach(SidebarSection.allCases) { section in
                Label(section.title, systemImage: section.symbol)
                    .tag(section)
            }
        }
        .listStyle(.sidebar)
        .navigationSplitViewColumnWidth(min: 170, ideal: 190, max: 240)
    }
}
