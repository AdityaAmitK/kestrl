import AppKit
import SwiftUI

/// The smallest a pane can be dragged to, in points.
private let minimumPaneSize: CGFloat = 160

/// Two panes with a draggable divider. The position is kept between launches under `autosaveName`.
/// SwiftUI's split views do not remember their divider, so this wraps `NSSplitView`.
struct AutosavingSplit<First: View, Second: View>: NSViewRepresentable {
    let autosaveName: String
    /// `.horizontal` puts the panes side by side; `.vertical` stacks them.
    let axis: Axis
    /// The first pane's share of the space until the user moves the divider
    let defaultFraction: CGFloat
    /// The narrowest the first pane can be dragged, and the least it starts with
    let firstPaneMinimum: CGFloat
    let first: First
    let second: Second

    init(
        autosaveName: String,
        axis: Axis,
        defaultFraction: CGFloat,
        firstPaneMinimum: CGFloat = minimumPaneSize,
        @ViewBuilder first: () -> First,
        @ViewBuilder second: () -> Second
    ) {
        self.autosaveName = autosaveName
        self.axis = axis
        self.defaultFraction = defaultFraction
        self.firstPaneMinimum = firstPaneMinimum
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
        let firstHost = NSHostingView(rootView: first)
        firstHost.sizingOptions = []
        let secondHost = NSHostingView(rootView: second)
        secondHost.sizingOptions = []
        split.addSubview(firstHost)
        split.addSubview(secondHost)
        context.coordinator.firstPaneMinimum = firstPaneMinimum
        context.coordinator.firstHost = firstHost
        context.coordinator.secondHost = secondHost

        split.autosaveName = autosaveName
        let savedKey = "NSSplitView Subview Frames \(autosaveName)"
        let defaultFraction = defaultFraction
        let firstPaneMinimum = firstPaneMinimum
        DispatchQueue.main.async {
            // Only a first launch lays out at the default; later launches keep the saved divider.
            guard UserDefaults.standard.object(forKey: savedKey) == nil else { return }
            split.layoutSubtreeIfNeeded()
            let total = split.isVertical ? split.bounds.width : split.bounds.height
            split.setPosition(max(total * defaultFraction, firstPaneMinimum), ofDividerAt: 0)
        }
        return split
    }

    func updateNSView(_ split: NSSplitView, context: Context) {
        context.coordinator.firstHost?.rootView = first
        context.coordinator.secondHost?.rootView = second
    }

    @MainActor
    final class Coordinator: NSObject, NSSplitViewDelegate {
        var firstHost: NSHostingView<First>?
        var secondHost: NSHostingView<Second>?
        var firstPaneMinimum = minimumPaneSize

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
            proposedMaximumPosition - minimumPaneSize
        }
    }
}
