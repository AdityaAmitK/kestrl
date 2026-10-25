import SwiftUI

/// Quiet pane furniture shared by the workbench. Content gets the space; controls stay compact.
struct PaneHeader<Trailing: View>: View {
    let title: String
    let subtitle: String?
    @ViewBuilder var trailing: Trailing

    init(_ title: String, subtitle: String? = nil, @ViewBuilder trailing: () -> Trailing = { EmptyView() }) {
        self.title = title
        self.subtitle = subtitle
        self.trailing = trailing()
    }

    var body: some View {
        HStack(spacing: 8) {
            Text(title).font(.system(size: 12, weight: .semibold))
            if let subtitle {
                Text(subtitle).font(.system(size: 11)).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer(minLength: 8)
            trailing
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .frame(height: 36)
        .background(.bar)
    }
}

/// A compact empty state, sized for a developer tool rather than a full-screen onboarding page.
struct EmptyPane: View {
    let title: String
    let symbol: String

    init(_ title: String, systemImage: String, description: Text? = nil) {
        self.title = title
        self.symbol = systemImage
        self.description = description
    }

    private let description: Text?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Image(systemName: symbol).font(.system(size: 22, weight: .light)).foregroundStyle(.tertiary)
                .padding(.bottom, 4)
            Text(title).font(.system(size: 13, weight: .semibold))
            description?.font(.system(size: 12)).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: 320, alignment: .leading)
        .padding(24)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
