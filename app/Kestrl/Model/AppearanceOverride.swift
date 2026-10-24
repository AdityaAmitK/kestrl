import SwiftUI

/// Forces light or dark appearance in debug builds only, so both can be screenshotted. Release builds follow the system.
enum AppearanceOverride: String, CaseIterable, Identifiable {
    case system
    case light
    case dark

    var id: Self { self }

    var title: String {
        switch self {
        case .system: return "System"
        case .light: return "Light"
        case .dark: return "Dark"
        }
    }

    var colorScheme: ColorScheme? {
        #if DEBUG
        switch self {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
        #else
        return nil
        #endif
    }
}
