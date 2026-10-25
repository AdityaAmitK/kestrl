import SwiftUI

/// Edit the selected profile: headers, query parameters, and when it applies automatically.
struct ProfileDetailView: View {
    @Bindable var store: WorkspaceStore
    @State private var part: Part = .headers

    private enum Part: String, CaseIterable, Identifiable {
        case headers = "Headers"
        case query = "Query"
        case conditions = "Applies"
        var id: Self { self }
    }

    private var editor: ProfileEditor { store.profileEditor }

    var body: some View {
        @Bindable var editor = editor
        if let name = editor.loadedProfile, name == editor.inspected {
            VStack(spacing: 0) {
                PaneHeader(name, subtitle: subtitle) {
                    if editor.hasChanges {
                        Button("Revert") { editor.revert() }
                            .help("Discard unsaved changes")
                    }
                    Group {
                        if editor.hasChanges {
                            saveButton(name).buttonStyle(.borderedProminent)
                        } else {
                            saveButton(name).buttonStyle(.bordered)
                        }
                    }
                }
                Divider()
                HStack {
                    Picker("Profile part", selection: $part) {
                        ForEach(Part.allCases) { Text(title(for: $0)).tag($0) }
                    }
                    .pickerStyle(.segmented).labelsHidden().fixedSize()
                    Spacer()
                }
                .controlSize(.small).padding(.horizontal, 12).padding(.vertical, 7)
                Divider()
                switch part {
                case .headers:
                    KeyValueLinesEditor(
                        text: $editor.draft.headers, separator: ":", keyPlaceholder: "header", valuePlaceholder: "value",
                        variables: store.editorVariables, environmentName: store.selectedEnvironment
                    )
                    .background(Color(nsColor: .textBackgroundColor))
                case .query:
                    KeyValueLinesEditor(
                        text: $editor.draft.query, separator: "=", keyPlaceholder: "parameter", valuePlaceholder: "value",
                        variables: store.editorVariables, environmentName: store.selectedEnvironment
                    )
                    .background(Color(nsColor: .textBackgroundColor))
                case .conditions:
                    conditions($editor.draft)
                }
                if let message = editor.validationMessage {
                    Divider()
                    Label(message, systemImage: "exclamationmark.triangle")
                        .foregroundStyle(.orange)
                        .font(.callout)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, 12)
                        .frame(height: 32)
                }
            }
        } else {
            EmptyPane(
                "Select a profile",
                systemImage: "slider.horizontal.3",
                description: Text("Its headers, query parameters and conditions appear here, ready to edit.")
            )
        }
    }

    private func saveButton(_ name: String) -> some View {
        Button(editor.isSaving ? "Saving…" : "Save") {
            Task { await editor.save() }
        }
        .keyboardShortcut("s", modifiers: .command)
        .disabled(!editor.hasChanges || editor.validationMessage != nil || editor.isSaving)
        .help("Save to .kestrl/profiles/\(name).toml (⌘S)")
    }

    private var subtitle: String {
        var parts = [editor.draft.isAutomatic ? "Automatic" : "On request"]
        if editor.hasChanges { parts.append("Edited") }
        return parts.joined(separator: " · ")
    }

    private func title(for part: Part) -> String {
        let count: (String) -> Int = { $0.split(separator: "\n").filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }.count }
        switch part {
        case .headers: return count(editor.draft.headers) > 0 ? "Headers \(count(editor.draft.headers))" : "Headers"
        case .query: return count(editor.draft.query) > 0 ? "Query \(count(editor.draft.query))" : "Query"
        case .conditions: return editor.draft.isAutomatic ? "Applies ●" : "Applies"
        }
    }

    private func conditions(_ draft: Binding<ProfileDraft>) -> some View {
        Form {
            Section {
                Toggle("Apply automatically", isOn: draft.isAutomatic)
                    .accessibilityLabel("Apply automatically")
                Text(draft.wrappedValue.isAutomatic
                     ? "Added to every request that matches all conditions below, in addition to profiles a request asks for."
                     : "Added only when a request lists this profile (Options › Profiles) or you pass -p on the CLI.")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if draft.wrappedValue.isAutomatic {
                Section("Environments") {
                    ForEach(store.workspace?.envs ?? [], id: \.self) { env in
                        Toggle(env, isOn: Binding(
                            get: { draft.wrappedValue.environments.contains(env) },
                            set: { isOn in
                                if isOn { draft.wrappedValue.environments.insert(env) } else { draft.wrappedValue.environments.remove(env) }
                            }
                        ))
                    }
                    Text("None selected means any environment.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Section("URL pattern") {
                    TextField("URL pattern", text: draft.urlPattern, prompt: Text("*/admin/*"))
                        .font(.body.monospaced())
                        .labelsHidden()
                        .accessibilityLabel("URL pattern")
                    Text("* matches any text. Leave empty to match every URL.")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
    }
}
