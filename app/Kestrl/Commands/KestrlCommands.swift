import SwiftUI

/// Menu commands. Every shortcut shown in a menu is registered here.
struct KestrlCommands: Commands {
    @Bindable var store: WorkspaceStore

    var body: some Commands {
        CommandGroup(replacing: .newItem) {
            Button("New Request") {
                store.newRequest()
            }
            .keyboardShortcut("n", modifiers: .command)
            .disabled(store.workspace == nil)

            Button("Save request") {
                Task { await store.saveRequestDraftAsFile() }
            }
            .keyboardShortcut("s", modifiers: .command)
            .disabled(store.section != .requests || !store.isDraftModified)

            Button("Save request as…") {
                Task { await store.saveRequestDraftAsFile(asCopy: true) }
            }
            .keyboardShortcut("s", modifiers: [.command, .shift])
            .disabled(store.section != .requests)

            Divider()

            Button("New Workspace…") {
                store.isShowingNewPanel = true
            }
            .keyboardShortcut("n", modifiers: [.command, .shift])

            Button("Open workspace…") {
                store.isShowingOpenPanel = true
            }
            .keyboardShortcut("o", modifiers: .command)
        }

        CommandMenu("Request") {
            Button("Send") {
                Task { await store.sendCurrentRequest() }
            }
            .keyboardShortcut(.return, modifiers: .command)
            .disabled(!store.canSendRequest)

            Divider()

            Button("Copy response JSON") {
                store.copyShownJSON()
            }
            .keyboardShortcut("c", modifiers: [.command, .shift])
            .disabled(!store.hasShownExchange)
        }

        CommandGroup(after: .sidebar) {
            Toggle("Wrap log lines", isOn: $store.wrapLogLines)
                .keyboardShortcut("w", modifiers: [.command, .option])

            Divider()

            Button("Refresh") {
                Task { await store.reload() }
            }
            .keyboardShortcut("r", modifiers: .command)
            .disabled(store.workspace == nil)

            #if DEBUG
            Divider()

            Menu("Appearance") {
                Picker("Appearance", selection: $store.appearance) {
                    ForEach(AppearanceOverride.allCases) { option in
                        Text(option.title).tag(option)
                    }
                }
                .pickerStyle(.inline)
            }
            #endif
        }
    }
}
