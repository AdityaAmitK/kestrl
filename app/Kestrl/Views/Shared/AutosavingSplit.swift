import AppKit
import SwiftUI

/// The smallest a pane can be dragged to, in points.
private let minimumPaneSize: CGFloat = 160

/// The width range of a content column: it starts at `ideal` and can be dragged within `minimum...maximum`.
struct PaneWidth {
    let minimum: CGFloat
    let ideal: CGFloat
    let maximum: CGFloat

    /// The list column beside a detail pane. Wide enough for names, narrow enough to leave the detail room.
    static let content = PaneWidth(minimum: 200, ideal: 240, maximum: 340)
}

/// Two panes with a draggable divider. The position is kept between launches under `autosaveName`.
/// SwiftUI's split views do not remember their divider, so this wraps `NSSplitView`.
struct AutosavingSplit<First: View, Second: View>: NSViewRepresentable {
    let autosaveName: String
    /// `.horizontal` puts the panes side by side; `.vertical` stacks them.
    let axis: Axis
    /// The first pane's share of the space until the user moves the divider. Used when `firstPaneWidth` is nil.
    let defaultFraction: CGFloat
    /// The first pane's width range for a horizontal split. Its width is clamped to this range.
    let firstPaneWidth: PaneWidth?
    let first: First
    let second: Second

    init(
        autosaveName: String,
        axis: Axis,
        defaultFraction: CGFloat = 0.5,
        firstPaneWidth: PaneWidth? = nil,
        @ViewBuilder first: () -> First,
        @ViewBuilder second: () -> Second
    ) {
        self.autosaveName = autosaveName
        self.axis = axis
        self.defaultFraction = defaultFraction
        self.firstPaneWidth = firstPaneWidth
        self.first = first()
        self.second = second()
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> NSSplitView {
        let split = NSSplitView()
        split.isVertical = axis == .horizontal
        split.dividerStyle = .thin
        split.delegate = context.coordinator

        // Panes take the space the split gives them, not the size of their content.
        let firstHost = NSHostingView(rootView: AnyView(first.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)))
        firstHost.sizingOptions = []
        let secondHost = NSHostingView(rootView: AnyView(second.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)))
        secondHost.sizingOptions = []
        split.addSubview(firstHost)
        split.addSubview(secondHost)
        context.coordinator.firstPaneMinimum = firstPaneWidth?.minimum ?? minimumPaneSize
        context.coordinator.firstPaneMaximum = firstPaneWidth?.maximum ?? .greatestFiniteMagnitude
        context.coordinator.firstHost = firstHost
        context.coordinator.secondHost = secondHost

        let savedKey = "NSSplitView Subview Frames \(autosaveName)"
        context.coordinator.initialFraction = defaultFraction
        context.coordinator.initialWidth = firstPaneWidth?.ideal
        context.coordinator.needsInitialPosition = UserDefaults.standard.object(forKey: savedKey) == nil
        split.autosaveName = autosaveName
        return split
    }

    func updateNSView(_ split: NSSplitView, context: Context) {
        context.coordinator.firstHost?.rootView = AnyView(first.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading))
        context.coordinator.secondHost?.rootView = AnyView(second.frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading))
    }

    @MainActor
    final class Coordinator: NSObject, NSSplitViewDelegate {
        var firstHost: NSHostingView<AnyView>?
        var secondHost: NSHostingView<AnyView>?
        var firstPaneMinimum = minimumPaneSize
        var firstPaneMaximum = CGFloat.greatestFiniteMagnitude
        var initialFraction: CGFloat = 0.5
        var initialWidth: CGFloat?
        var needsInitialPosition = false
        private var isPositioning = false

        func splitViewDidResizeSubviews(_ notification: Notification) {
            guard !isPositioning, let split = notification.object as? NSSplitView,
                  let first = split.subviews.first, split.subviews.count == 2 else { return }
            let total = split.isVertical ? split.bounds.width : split.bounds.height
            guard total >= firstPaneMinimum + minimumPaneSize + split.dividerThickness else { return }
            let current = split.isVertical ? first.frame.width : first.frame.height
            let proposed = needsInitialPosition ? (initialWidth ?? total * initialFraction) : current
            let maximum = min(firstPaneMaximum, total - minimumPaneSize - split.dividerThickness)
            let position = min(max(proposed, firstPaneMinimum), maximum)
            needsInitialPosition = false
            guard abs(position - current) > 0.5 else { return }
            isPositioning = true
            defer { isPositioning = false }
            split.setPosition(position, ofDividerAt: 0)
        }

        func splitView(
            _ splitView: NSSplitView,
            constrainMinCoordinate proposedMinimumPosition: CGFloat,
            ofSubviewAt dividerIndex: Int
        ) -> CGFloat {
            proposedMinimumPosition + (dividerIndex == 0 ? firstPaneMinimum : minimumPaneSize)
        }

        func splitView(
            _ splitView: NSSplitView,
            constrainMaxCoordinate proposedMaximumPosition: CGFloat,
            ofSubviewAt dividerIndex: Int
        ) -> CGFloat {
            guard dividerIndex == 0 else { return proposedMaximumPosition - minimumPaneSize }
            return min(proposedMaximumPosition - minimumPaneSize, firstPaneMaximum)
        }
    }
}
