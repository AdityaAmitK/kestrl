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

            Divider()

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
