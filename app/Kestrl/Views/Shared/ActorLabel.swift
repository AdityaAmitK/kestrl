import SwiftUI

/// Who made an event, with a symbol that tells people from agents at a glance.
struct ActorLabel: View {
    let actor: String

    var body: some View {
        Label(actor, systemImage: symbol)
    }

    private var symbol: String {
        switch actor {
        case "me": return "person"
        case "cursor", "claude-code", "codex": return "sparkles"
        default: return "cpu"
        }
    }
}
