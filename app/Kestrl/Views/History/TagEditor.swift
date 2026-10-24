import SwiftUI

/// Tag chips with remove buttons, and a field to add one. A leading `#` is optional.
struct TagEditor: View {
    let store: WorkspaceStore
    let tags: [String]

    @State private var draft = ""

    var body: some View {
        HStack(spacing: 6) {
            ForEach(tags, id: \.self) { tag in
                HStack(spacing: 4) {
                    Text("#\(tag)")
                    Button {
                        Task { await store.removeTag(tag) }
                    } label: {
                        Image(systemName: "xmark.circle.fill")
                    }
                    .buttonStyle(.plain)
                    .foregroundStyle(.secondary)
                    .help("Remove tag #\(tag)")
                    .accessibilityLabel("Remove tag \(tag)")
                }
                .font(.callout)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(.quaternary, in: Capsule())
            }

            TextField("Add tag", text: $draft)
                .textFieldStyle(.plain)
                .frame(width: 90)
                .onSubmit {
                    let tag = draft.trimmingCharacters(in: .whitespaces)
                    guard !tag.isEmpty else { return }
                    draft = ""
                    Task { await store.addTag(tag) }
                }
        }
    }
}
