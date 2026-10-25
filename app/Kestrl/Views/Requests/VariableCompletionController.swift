import AppKit

/// A nonactivating caret panel: clicking a name never takes keyboard focus from the editor.
@MainActor
final class VariableCompletionController: NSObject, NSTableViewDataSource, NSTableViewDelegate {
    private weak var editor: VariableEditorTextView?
    private let panel = VariableCompletionPanel(contentRect: .zero, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    private let table = NSTableView()
    private let scroll = NSScrollView()
    private var candidates: [VariableInfo] = []
    private var replacement = NSRange(location: 0, length: 0)
    private var dismissedContext: String?
    private var context = ""
    private var environmentName: String?

    var isVisible: Bool { panel.isVisible }

    override init() {
        super.init()
        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("variable"))
        column.width = 416
        table.addTableColumn(column)
        table.headerView = nil
        table.rowHeight = 48
        table.intercellSpacing = .zero
        table.backgroundColor = .clear
        table.dataSource = self
        table.delegate = self
        table.target = self
        table.action = #selector(choose)
        table.setAccessibilityLabel("Variable completions")
        scroll.documentView = table
        scroll.hasVerticalScroller = true
        scroll.scrollerStyle = .overlay
        scroll.autohidesScrollers = true
        let surface = NSVisualEffectView()
        surface.material = .popover
        surface.state = .active
        surface.blendingMode = .withinWindow
        surface.wantsLayer = true
        surface.layer?.cornerRadius = 10
        surface.layer?.masksToBounds = true
        surface.layer?.borderWidth = 1
        surface.layer?.borderColor = NSColor.separatorColor.withAlphaComponent(0.35).cgColor
        surface.addSubview(scroll)
        scroll.drawsBackground = false
        panel.contentView = surface
        panel.hasShadow = true
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.level = .popUpMenu
        panel.hidesOnDeactivate = true
        panel.becomesKeyOnlyIfNeeded = true
        panel.isReleasedWhenClosed = false
    }

    func update(_ view: VariableEditorTextView, variables: [VariableInfo], environmentName: String?) {
        self.environmentName = environmentName
        editor = view
        guard view.window?.firstResponder === view, !view.hasMarkedText(), view.selectedRange().length == 0 else {
            dismiss()
            return
        }
        let caret = view.selectedRange().location
        let ns = view.string as NSString
        let prefix = ns.substring(to: caret)
        guard let opening = prefix.range(of: "{{", options: .backwards) else { dismissedContext = nil; dismiss(); return }
        let fragment = String(prefix[opening.upperBound...])
        guard !fragment.contains(where: { $0 == "{" || $0 == "}" || $0.isWhitespace }) else { dismissedContext = nil; dismiss(); return }
        let start = (prefix[..<opening.lowerBound] as NSString).length
        let suffix = ns.substring(from: caret)
        let tail = String(suffix.prefix { $0.isLetter || $0.isNumber || "_-.:".contains($0) })
        let afterTail = String(suffix.dropFirst(tail.count))
        // A caret moved into an already closed token doesn't open the list.
        guard !afterTail.hasPrefix("}}") else { dismissedContext = nil; dismiss(); return }
        context = "\(start):\(caret):\(fragment)"
        guard context != dismissedContext else { dismiss(); return }
        replacement = NSRange(location: start, length: caret - start + (tail as NSString).length)
        candidates = variables.filter { $0.name.lowercased().hasPrefix(fragment.lowercased()) }.sorted { $0.name < $1.name }
        guard !candidates.isEmpty, let window = view.window else { dismissedContext = nil; dismiss(); return }
        table.reloadData()
        table.selectRowIndexes(IndexSet(integer: 0), byExtendingSelection: false)
        scroll.hasVerticalScroller = candidates.count > 7
        let height = CGFloat(min(candidates.count, 7)) * table.rowHeight + 12
        var actual = NSRange()
        let caretRect = view.firstRect(forCharacterRange: NSRange(location: caret, length: 0), actualRange: &actual)
        panel.appearance = view.effectiveAppearance
        let screen = window.screen?.visibleFrame ?? window.frame
        let width = min(CGFloat(440), screen.width)
        let x = min(max(caretRect.minX, screen.minX), screen.maxX - width)
        let y = caretRect.minY - height >= screen.minY ? caretRect.minY - height : caretRect.maxY
        panel.setFrame(NSRect(x: x, y: y, width: width, height: height), display: true)
        scroll.frame = NSRect(x: 0, y: 6, width: width, height: height - 12)
        table.frame.size.width = width
        table.tableColumns.first?.width = width - 20
        if panel.parent !== window { window.addChildWindow(panel, ordered: .above) }
        panel.orderFront(nil)
    }

    func dismiss() {
        panel.parent?.removeChildWindow(panel)
        panel.orderOut(nil)
    }

    func handle(_ event: NSEvent) -> Bool {
        guard panel.isVisible, event.modifierFlags.intersection([.command, .control, .option]).isEmpty else { return false }
        switch event.keyCode {
        case 125, 126:
            let row = (table.selectedRow + (event.keyCode == 125 ? 1 : -1) + candidates.count) % candidates.count
            table.selectRowIndexes(IndexSet(integer: row), byExtendingSelection: false)
            table.scrollRowToVisible(row)
        case 36, 48: choose()
        case 53:
            dismissedContext = context
            dismiss()
        default: return false
        }
        return true
    }

    @objc private func choose() {
        let row = table.selectedRow
        guard candidates.indices.contains(row), let editor else { return }
        let token = "{{\(candidates[row].name)}}"
        dismiss()
        editor.insertText(token, replacementRange: replacement)
    }

    func numberOfRows(in tableView: NSTableView) -> Int { candidates.count }

    func tableView(_ tableView: NSTableView, rowViewForRow row: Int) -> NSTableRowView? {
        VariableCompletionRow()
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        VariableCompletionCell(info: candidates[row], environmentName: environmentName)
    }
}
