import SwiftUI

/// One event: its summary, tags, note, then the response. The request sits in a disclosure above the response.
struct EventDetailView: View {
    let store: WorkspaceStore
    let shown: ShownEvent

    @State private var noteDraft = ""

    var body: some View {
        let event = shown.detail.event
        let http = shown.detail.http
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    if let http {
                        MethodBadge(method: http.method)
                        Text(http.url)
                            .font(.body.monospaced())
                            .lineLimit(2)
                            .textSelection(.enabled)
                    } else {
                        Text(event.summary)
                            .font(.body.monospaced())
                            .lineLimit(2)
                    }
                }

                HStack(spacing: 12) {
                    Text(TimeFormat.absolute(ms: event.ts))
                    ActorLabel(actor: event.actor)
                    Text(event.source)
                }
                .font(.caption)
                .foregroundStyle(.secondary)

                TagEditor(store: store, tags: event.tags)

                HStack(alignment: .top, spacing: 8) {
                    TextField("Note", text: $noteDraft, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(1...4)
                        .onSubmit(saveNote)
                    Button("Save note", action: saveNote)
                        .disabled(noteDraft == (shown.detail.note ?? ""))
                }
            }
            .padding(16)

            Divider()

            if let http {
                DisclosureGroup("Request") {
                    VStack(alignment: .leading, spacing: 10) {
                        KeyValueGrid(title: "Headers", pairs: http.reqHeaders)
                        if let body = shown.requestBody {
                            JSONTextView(text: body.text)
                                .frame(height: 140)
                                .clipShape(RoundedRectangle(cornerRadius: 6))
                        }
                    }
                    .padding(.top, 6)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 8)

                Divider()

                HStack(spacing: 12) {
                    StatusBadge(status: http.status, error: http.error)
                    Text("\(http.durationMs) ms")
                        .monospacedDigit()
                        .foregroundStyle(.secondary)
                    Spacer()
                    CopyJSONButton(store: store)
                }
                .padding(.horizontal, 16)
                .padding(.vertical, 12)

                ExchangeInspector(
                    headers: http.resHeaders,
                    payload: shown.responseBody,
                    emptyMessage: http.error ?? "The response had no body."
                )
            } else {
                EmptyPane(
                    "No HTTP exchange",
                    systemImage: "doc",
                    description: Text(event.summary)
                )
            }
        }
        .task(id: shown.detail.event.id) {
            noteDraft = shown.detail.note ?? ""
        }
    }

    private func saveNote() {
        let note = noteDraft
        Task { await store.saveNote(note) }
    }
}
