import SwiftUI

/// The diff of two HTTP events, shown as pretty JSON.
struct CompareView: View {
    let store: WorkspaceStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text("Compare events")
                    .font(.headline)
                Spacer()
                Button("Done") {
                    store.isShowingComparison = false
                }
                .keyboardShortcut(.defaultAction)
            }
            .padding(16)

            Divider()

            if let comparison = store.comparison {
                JSONTextView(text: comparison.text)
            }
        }
        .frame(minWidth: 640, minHeight: 480)
    }
}
