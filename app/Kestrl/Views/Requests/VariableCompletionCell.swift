import AppKit

/// A completion shows the name first, then its safe resolved preview.
final class VariableCompletionCell: NSTableCellView {
    init(info: VariableInfo, environmentName: String?) {
        super.init(frame: .zero)
        let icon = NSImageView()
        icon.image = NSImage(systemSymbolName: info.isSecret ? "key.fill" : "curlybraces", accessibilityDescription: info.isSecret ? "Secret" : "Variable")
        icon.contentTintColor = info.isSecret ? .systemPurple : .secondaryLabelColor
        let name = NSTextField(labelWithString: info.name)
        name.font = .monospacedSystemFont(ofSize: 12, weight: .semibold)
        let source = NSTextField(labelWithString: info.source == "environment" ? (environmentName ?? "environment") : info.source)
        source.font = .systemFont(ofSize: 10, weight: .medium)
        source.textColor = .secondaryLabelColor
        let preview = info.isSecret ? "Secret · \(info.isSet ? "set" : "not set")" : (info.value.isEmpty ? "(empty)" : info.value)
        let value = NSTextField(labelWithString: preview)
        value.font = .monospacedSystemFont(ofSize: 11, weight: .regular)
        value.textColor = .secondaryLabelColor
        value.lineBreakMode = .byTruncatingMiddle
        name.lineBreakMode = .byTruncatingTail
        let children: [NSView] = [icon, name, source, value]
        for child in children {
            child.translatesAutoresizingMaskIntoConstraints = false
            addSubview(child)
        }
        source.setContentCompressionResistancePriority(.required, for: .horizontal)
        NSLayoutConstraint.activate([
            icon.leadingAnchor.constraint(equalTo: leadingAnchor, constant: 12),
            icon.centerYAnchor.constraint(equalTo: centerYAnchor),
            icon.widthAnchor.constraint(equalToConstant: 16),
            icon.heightAnchor.constraint(equalToConstant: 16),
            name.leadingAnchor.constraint(equalTo: icon.trailingAnchor, constant: 10),
            name.topAnchor.constraint(equalTo: topAnchor, constant: 7),
            source.leadingAnchor.constraint(greaterThanOrEqualTo: name.trailingAnchor, constant: 12),
            source.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            source.firstBaselineAnchor.constraint(equalTo: name.firstBaselineAnchor),
            value.leadingAnchor.constraint(equalTo: name.leadingAnchor),
            value.trailingAnchor.constraint(equalTo: trailingAnchor, constant: -12),
            value.topAnchor.constraint(equalTo: name.bottomAnchor, constant: 3),
        ])
        toolTip = info.isSecret ? preview : "\(source.stringValue): \(info.value)"
        setAccessibilityElement(true)
        setAccessibilityLabel("\(info.name), \(preview), \(source.stringValue)")
    }

    required init?(coder: NSCoder) { return nil }
}
