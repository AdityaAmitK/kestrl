import AppKit
import Foundation
import Observation

/// The open workspace and the state every window shows. FFI calls run off the main actor through `KestrlBridge`.
@Observable
@MainActor
final class WorkspaceStore {
    private static let lastWorkspaceKey = "lastWorkspacePath"
    private static let sectionKey = "selectedSection"
    private static let appearanceKey = "debugAppearance"
    private static let wrapLogLinesKey = "wrapLogLines"

    private let defaults: UserDefaults

    private(set) var workspace: WorkspaceInfo?
    private(set) var requestTree: [RequestNode] = []
    private(set) var requestDetail: RequestDetail?
    private(set) var requestBody: PreparedBody?
    private(set) var secrets: [SecretStatus] = []
    private(set) var resolvedVariables: [KeyValue] = []
    private(set) var events: [EventSummary] = []
    private(set) var knownActors: [String] = []
    private(set) var eventDetail: ShownEvent?
    private(set) var comparison: PreparedBody?
    private(set) var response: ShownResponse?
    private(set) var isSending = false
    private(set) var justCopied = false
    private(set) var errorMessage: String?
    /// Folders the user collapsed in the request tree. Folders are expanded unless listed here.
    private(set) var collapsedFolders: Set<String> = []

    private(set) var services: [ServiceInfo] = []
    /// The newest `logCapacity` lines of the selected service, oldest first
    private(set) var logLines: [LogLine] = []
    /// Why the stream stopped, when it did
    private(set) var logStatus: String?
    /// The filter the stream is running with. nil shows every line.
    private(set) var logFilter: String?
    private(set) var logFilterError: String?

    private(set) var connections: [String] = []
    /// Tables of each expanded connection, keyed by connection name
    private(set) var tablesByConnection: [String: [TableInfo]] = [:]
    private(set) var expandedConnections: Set<String> = []
    private(set) var tableColumns: [ColumnInfo] = []
    private(set) var queryResult: ShownQueryResult?
    private(set) var isRunningQuery = false

    var section: SidebarSection {
        didSet { defaults.set(section.rawValue, forKey: Self.sectionKey) }
    }
    var appearance: AppearanceOverride {
        didSet { defaults.set(appearance.rawValue, forKey: Self.appearanceKey) }
    }
    /// Whether long log lines wrap. Off, each line stays on one row and the log scrolls sideways.
    var wrapLogLines: Bool {
        didSet { defaults.set(wrapLogLines, forKey: Self.wrapLogLinesKey) }
    }
    var selectedRequest: String?
    /// The environment requests are sent with
    var selectedEnvironment: String?
    /// The environment whose variables are listed
    var inspectedEnvironment: String?
    var selectedSecret: String?
    var selectedEventIDs: Set<String> = []
    var historyText = ""
    var historyActor: String?
    private var requestDrafts: [String: RequestEditorDraft] = [:]
    private var originalDrafts: [String: RequestEditorDraft] = [:]
    private var draftKey: String { "\(workspace?.root ?? "")|\(selectedRequest ?? "<new>")" }
    var requestDraft: RequestEditorDraft {
        get { requestDrafts[draftKey] ?? RequestEditorDraft() }
        set { requestDrafts[draftKey] = newValue }
    }
    var isDraftModified: Bool { requestDraft != (originalDrafts[draftKey] ?? RequestEditorDraft()) }
    var hasUnsavedDrafts: Bool {
        requestDrafts.contains { key, draft in draft != (originalDrafts[key] ?? RequestEditorDraft()) }
    }
    var adHocMethod: String {
        get { requestDraft.method }
        set { requestDraft.method = newValue }
    }
    var adHocURL: String {
        get { requestDraft.url }
        set { requestDraft.url = newValue }
    }
    var isShowingNewPanel = false
    var isShowingOpenPanel = false
    var isShowingComparison = false
    var isSettingSecret = false
    var selectedLogService: String?
    var selectedLogSeqs: Set<UInt64> = []
    /// Where a shift-click range starts
    private var logAnchor: UInt64?
    /// What the user has typed in the log filter. It applies after a short pause, once it is valid.
    var logFilterText = ""
    /// Whether the log view follows new lines. Scrolling up pauses it.
    var logsFollowing = true
    var selectedDatabaseItem: DatabaseSelection?
    var sql = ""

    private var logFilterTask: Task<Void, Never>?
    private static let logCapacity = 10_000

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        section = SidebarSection(rawValue: defaults.string(forKey: Self.sectionKey) ?? "") ?? .requests
        appearance = AppearanceOverride(rawValue: defaults.string(forKey: Self.appearanceKey) ?? "") ?? .system
        wrapLogLines = defaults.bool(forKey: Self.wrapLogLinesKey)
    }

    var canSendAdHoc: Bool {
        workspace != nil && !isSending && !adHocURL.trimmingCharacters(in: .whitespaces).isEmpty
    }

    /// Whether ⌘↩ and the Send buttons can send: the open request file, or the ad-hoc URL when none is open.
    var canSendRequest: Bool {
        section == .requests && canSendAdHoc
    }

    var hasShownExchange: Bool {
        switch section {
        case .requests: return response != nil
        case .history: return eventDetail?.detail.http != nil
        case .capture, .logs, .database, .environments, .secrets: return false
        }
    }

    /// Changes whenever the history filter does, so the history view can restart its refresh loop.
    var historyQueryKey: String { "\(historyActor ?? "")|\(historyText)" }

    /// Changes whenever the service or its filter does, so the log view restarts its stream.
    var logStreamKey: String { "\(selectedLogService ?? "")|\(logFilter ?? "")" }

    /// The connection the Database section works on: the selected connection, or the one a table belongs to.
    var selectedConnection: String? { selectedDatabaseItem?.connection }

    /// The table the Database section has selected, if one is
    var selectedTable: DatabaseTable? {
        if case .table(let table) = selectedDatabaseItem { return table }
        return nil
    }

    func dismissError() {
        errorMessage = nil
    }

    func report(_ error: Error) {
        errorMessage = ErrorText.message(for: error)
    }

    // MARK: Workspace

    /// Choose or create a folder, initialize it, and select its starter request.
    func chooseNewWorkspace() async {
        let panel = NSOpenPanel()
        panel.title = "New workspace"
        panel.prompt = "Create"
        panel.message = "Choose a folder for .kestrl configuration and requests."
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        guard await panel.begin() == .OK, let url = panel.url else { return }
        await createWorkspaceHere(url)
    }

    /// Create through the shared core before opening the starter request.
    private func createWorkspaceHere(_ url: URL) async {
        let path = url.path(percentEncoded: false)
        do {
            let info = try await KestrlBridge.run { try createWorkspace(path: path) }
            applyWorkspace(info)
            section = .requests
            selectedRequest = "requests/health.req.toml"
            await requestSelectionChanged()
        } catch {
            report(error)
        }
    }

    func chooseWorkspace() async {
        let panel = NSOpenPanel()
        panel.title = "Open workspace"
        panel.prompt = "Open workspace"
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        guard await panel.begin() == .OK, let url = panel.url else { return }
        if !FileManager.default.fileExists(atPath: url.appendingPathComponent(".kestrl").path) {
            let alert = NSAlert()
            alert.messageText = "This folder isn't a Kestrl workspace"
            alert.informativeText = "Create .kestrl configuration and a starter request in this folder."
            alert.addButton(withTitle: "Create workspace here")
            alert.addButton(withTitle: "Cancel")
            guard alert.runModal() == .alertFirstButtonReturn else { return }
            await createWorkspaceHere(url)
            return
        }
        await loadWorkspace(from: url)
    }

    func restoreLastWorkspace() async {
        guard workspace == nil, let path = defaults.string(forKey: Self.lastWorkspaceKey) else { return }
        await loadWorkspace(from: URL(fileURLWithPath: path))
    }

    func loadWorkspace(from url: URL) async {
        let path = url.path(percentEncoded: false)
        do {
            let info = try await KestrlBridge.run { try openWorkspace(path: path) }
            applyWorkspace(info)
        } catch {
            report(error)
        }
    }

    /// Apply workspace metadata consistently after opening or creating.
    private func applyWorkspace(_ info: WorkspaceInfo) {
        if workspace?.root != info.root {
            response = nil
            requestDetail = nil
            requestBody = nil
            eventDetail = nil
            selectedRequest = nil
            selectedEventIDs = []
            resetDatabase()
        }
        workspace = info
        requestTree = RequestNode.build(info.requests)
        defaults.set(info.root, forKey: Self.lastWorkspaceKey)
        if let selected = selectedRequest, !info.requests.contains(where: { $0.path == selected }) {
            selectedRequest = nil
        }
        if !info.envs.contains(selectedEnvironment ?? "") {
            selectedEnvironment = info.envs.contains(info.defaultEnv ?? "") ? info.defaultEnv : info.envs.first
        }
        errorMessage = nil
    }

    func reload() async {
        guard let root = workspace?.root else {
            await refreshEvents()
            return
        }
        await loadWorkspace(from: URL(fileURLWithPath: root))
        await refreshEvents()
        await loadSecrets()
        await loadResolvedVariables()
    }

    /// Loads what a section needs when it is chosen.
    func sectionChanged() async {
        switch section {
        case .requests:
            break
        case .history:
            await refreshEvents()
        case .capture:
            break
        case .environments:
            if inspectedEnvironment == nil { inspectedEnvironment = selectedEnvironment }
            await loadResolvedVariables()
        case .secrets:
            await loadSecrets()
        case .logs:
            await loadServices()
        case .database:
            await loadConnections()
        }
    }

    // MARK: Requests

    /// Folders start expanded. Only a folder the user collapsed is remembered.
    func setFolder(_ id: String, expanded: Bool) {
        if expanded {
            collapsedFolders.remove(id)
        } else {
            collapsedFolders.insert(id)
        }
    }

    func requestSelectionChanged() async {
        guard let root = workspace?.root, let path = selectedRequest else {
            requestDetail = nil
            requestBody = nil
            return
        }
        // Folder rows share the outline's selection; only request files can be read.
        guard workspace?.requests.contains(where: { $0.path == path }) == true else {
            selectedRequest = nil
            return
        }
        if response?.requestPath != path { response = nil }
        do {
            let detail = try await KestrlBridge.run { try readRequestFile(workspaceRoot: root, path: path) }
            guard selectedRequest == path else { return }
            requestDetail = detail
            let loaded = RequestEditorDraft(detail)
            if requestDrafts[draftKey] == nil { requestDraft = loaded }
            originalDrafts[draftKey] = loaded
            requestBody = await prepareOptional(detail.body)
        } catch {
            report(error)
        }
    }

    /// A captured exchange becomes an unresolved draft, never an automatic resend.
    func useCapturedRequest(_ exchange: CapturedExchange) {
        let newKey = "\(workspace?.root ?? "")|<new>"
        if let previous = requestDrafts[newKey], previous != RequestEditorDraft() {
            let alert = NSAlert()
            alert.messageText = "Replace the unsaved request draft?"
            alert.informativeText = "Save the existing draft first to keep it. The captured request opens without sending."
            alert.addButton(withTitle: "Keep draft")
            alert.addButton(withTitle: "Replace draft")
            guard alert.runModal() == .alertSecondButtonReturn else { return }
        }
        newRequest()
        var draft = RequestEditorDraft()
        draft.name = "Captured \(exchange.method)"
        draft.method = exchange.method
        draft.url = exchange.url
        let omitted = ["host", "content-length", "connection", "accept-encoding", "transfer-encoding"]
        draft.headers = exchange.requestHeaders.filter { !omitted.contains($0.key.lowercased()) }
            .map { "\($0.key): \($0.value)" }.joined(separator: "\n")
        if let body = exchange.requestBody, !body.isEmpty {
            draft.body = body
            draft.bodyKind = exchange.requestHeaders.contains {
                $0.key.lowercased() == "content-type" && $0.value.lowercased().contains("json")
            } ? "json" : "text"
        }
        requestDraft = draft
    }

    /// Sends the current draft without writing it to disk.
    func sendCurrentRequest() async {
        guard canSendRequest, let root = workspace?.root else { return }
        do {
            let values = [requestDraft.url, requestDraft.headers, requestDraft.query, requestDraft.body, requestDraft.profiles]
            guard !values.contains(where: { $0.contains("••••••") }) else {
                throw KestrlError.InvalidInput(message: "Replace masked values with {{secret:name}} references before sending this request.")
            }
            let draft = try requestDraft.record()
            let env = selectedEnvironment
            let path = selectedRequest
            let reference = isDraftModified ? nil : path
            await perform(requestPath: path) {
                try sendRequestDraft(workspaceRoot: root, draft: draft, env: env, requestPath: reference)
            }
        } catch { report(error) }
    }

    /// Returns to the unsaved draft. Existing edits are preserved when navigating away.
    func newRequest() {
        section = .requests
        selectedRequest = nil
        response = nil
    }

    func sendAdHocRequest() async { await sendCurrentRequest() }

    func discardRequestDraft() async {
        errorMessage = nil
        response = nil
        requestDrafts.removeValue(forKey: draftKey)
        if selectedRequest != nil { await requestSelectionChanged() }
    }

    /// Save only on explicit action, refusing external edits through the core binding.
    func saveRequestDraftAsFile(asCopy: Bool = false) async {
        guard let root = workspace?.root else { return }
        let editingKey = draftKey
        let snapshot = requestDraft
        var path = asCopy ? nil : selectedRequest
        var draft = snapshot
        if path == nil {
            let panel = NSSavePanel()
            panel.title = "Save request"
            panel.nameFieldStringValue = "request.req.toml"
            panel.directoryURL = URL(fileURLWithPath: root)
            panel.canCreateDirectories = true
            guard await panel.begin() == .OK, let url = panel.url else { return }
            let prefix = URL(fileURLWithPath: root).standardizedFileURL.path + "/"
            guard url.standardizedFileURL.path.hasPrefix(prefix) else {
                report(KestrlError.InvalidInput(message: "Choose a file inside this workspace."))
                return
            }
            path = String(url.standardizedFileURL.path.dropFirst(prefix.count))
            draft.sourceText = ""
        }
        guard let path else { return }
        do {
            let record = try draft.record()
            let saved = try await KestrlBridge.run { try saveRequestDraft(workspaceRoot: root, path: path, draft: record) }
            // Saving a request must not resolve unrelated environment secrets or refresh other panes.
            await loadWorkspace(from: URL(fileURLWithPath: root))
            guard workspace?.root == root else { return }
            let shouldSelect = draftKey == editingKey && requestDraft == snapshot
            let savedKey = "\(root)|\(path)"
            let loaded = RequestEditorDraft(saved)
            if let latest = requestDrafts[editingKey], latest != snapshot, editingKey == savedKey {
                var updated = latest
                updated.sourceText = saved.sourceText
                requestDrafts[savedKey] = updated
            } else {
                requestDrafts[savedKey] = loaded
                if editingKey != savedKey, requestDrafts[editingKey] == snapshot {
                    requestDrafts.removeValue(forKey: editingKey)
                }
            }
            originalDrafts[savedKey] = loaded
            if shouldSelect {
                selectedRequest = path
                requestDetail = saved
            }
        } catch { report(error) }
    }

    private func perform(requestPath: String?, _ work: @escaping @Sendable () throws -> SentExchange) async {
        guard !isSending else { return }
        let selectionKey = draftKey
        let snapshot = requestDraft
        isSending = true
        defer { isSending = false }
        do {
            let exchange = try await KestrlBridge.run(work)
            let body = await PreparedBody.prepare(exchange.body)
            if draftKey == selectionKey, requestDraft == snapshot {
                response = ShownResponse(requestPath: requestPath, exchange: exchange, body: body)
            }
            await refreshEvents()
        } catch {
            report(error)
        }
    }

    // MARK: History

    func refreshEvents() async {
        let filter = EventFilter(
            text: historyText.isEmpty ? nil : historyText,
            actor: historyActor,
            tag: nil,
            since: nil,
            sessionId: nil,
            limit: 500
        )
        do {
            let found = try await KestrlBridge.run { try listEvents(filter: filter) }
            events = found
            if historyText.isEmpty && historyActor == nil {
                knownActors = Set(found.map(\.actor)).sorted()
            }
            selectedEventIDs = selectedEventIDs.intersection(found.map(\.id))
        } catch {
            report(error)
        }
    }

    func eventSelectionChanged() async {
        guard selectedEventIDs.count == 1, let id = selectedEventIDs.first else {
            eventDetail = nil
            return
        }
        await loadEvent(id)
    }

    func addTag(_ tag: String) async {
        await changeSelectedEvent { id in try tagEvent(id: id, tag: tag) }
    }

    func removeTag(_ tag: String) async {
        await changeSelectedEvent { id in try untagEvent(id: id, tag: tag) }
    }

    func saveNote(_ note: String) async {
        await changeSelectedEvent { id in try setNote(id: id, note: note) }
    }

    func compareSelectedEvents() async {
        // `events` is newest first; the diff reads older to newer.
        let ids = Array(events.filter { selectedEventIDs.contains($0.id) }.map(\.id).reversed())
        guard ids.count == 2 else { return }
        let older = ids[0]
        let newer = ids[1]
        do {
            let json = try await KestrlBridge.run { try diffEvents(a: older, b: newer) }
            comparison = await PreparedBody.prepare(json)
            isShowingComparison = true
        } catch {
            report(error)
        }
    }

    private func loadEvent(_ id: String) async {
        do {
            let detail = try await KestrlBridge.run { try showEvent(id: id) }
            guard selectedEventIDs == [id] else { return }
            eventDetail = ShownEvent(
                detail: detail,
                requestBody: await prepareOptional(detail.http?.reqBody),
                responseBody: await prepareOptional(detail.http?.resBody)
            )
        } catch {
            report(error)
        }
    }

    private func changeSelectedEvent(_ change: @escaping @Sendable (String) throws -> Void) async {
        guard let id = eventDetail?.detail.event.id else { return }
        do {
            try await KestrlBridge.run { try change(id) }
            await loadEvent(id)
            await refreshEvents()
        } catch {
            report(error)
        }
    }

    // MARK: Environments and secrets

    func loadResolvedVariables() async {
        guard let root = workspace?.root, let env = inspectedEnvironment else {
            resolvedVariables = []
            return
        }
        do {
            resolvedVariables = try await KestrlBridge.run { try resolvedEnv(workspaceRoot: root, env: env) }
        } catch {
            report(error)
        }
    }

    func loadSecrets() async {
        guard let root = workspace?.root else {
            secrets = []
            return
        }
        do {
            let found = try await KestrlBridge.run { try secretStatus(workspaceRoot: root) }
            secrets = found.sorted { $0.name < $1.name }
        } catch {
            report(error)
        }
    }

    /// Returns true when the value was stored, so the sheet knows to close.
    func saveSecret(name: String, value: String) async -> Bool {
        do {
            try await KestrlBridge.run { try setSecret(name: name, value: value) }
            await loadSecrets()
            return true
        } catch {
            report(error)
            return false
        }
    }

    func removeSecret(_ name: String) async {
        do {
            try await KestrlBridge.run { try deleteSecret(name: name) }
            await loadSecrets()
        } catch {
            report(error)
        }
    }

    // MARK: Logs

    func loadServices() async {
        do {
            services = try await KestrlBridge.run { try listServices() }
        } catch {
            report(error)
        }
    }

    /// Runs the stream for the selected service and its filter until the task is cancelled, which happens when the view goes away or either changes.
    func followSelectedService() async {
        guard let service = selectedLogService else { return }
        let filter = logFilter
        logLines = []
        selectedLogSeqs = []
        logAnchor = nil
        logStatus = nil
        logsFollowing = true

        let inbox = LogInbox()
        let follow = followLogs(service: service, grep: filter, sink: inbox)
        defer { follow.cancel() }
        while !Task.isCancelled {
            try? await Task.sleep(for: .milliseconds(100))
            let batch = inbox.drain()
            appendLogLines(batch.lines)
            if batch.ended {
                logStatus = batch.error ?? "The service stopped."
                return
            }
        }
    }

    private func appendLogLines(_ lines: [LogLine]) {
        guard !lines.isEmpty else { return }
        logLines.append(contentsOf: lines)
        if logLines.count > Self.logCapacity {
            logLines.removeFirst(logLines.count - Self.logCapacity)
        }
    }

    /// Waits briefly after typing, then applies the filter if it is a valid pattern. An invalid one keeps the last valid filter and shows why.
    func logFilterTextChanged() {
        logFilterTask?.cancel()
        logFilterTask = Task {
            try? await Task.sleep(for: .milliseconds(300))
            guard !Task.isCancelled else { return }
            applyLogFilter()
        }
    }

    private func applyLogFilter() {
        logFilterError = LogFilter.problem(with: logFilterText)
        guard logFilterError == nil else { return }
        logFilter = logFilterText.isEmpty ? nil : logFilterText
    }

    func selectLogLine(_ seq: UInt64, modifiers: NSEvent.ModifierFlags) {
        if modifiers.contains(.command) {
            if selectedLogSeqs.contains(seq) {
                selectedLogSeqs.remove(seq)
            } else {
                selectedLogSeqs.insert(seq)
            }
            logAnchor = seq
        } else if modifiers.contains(.shift), let anchor = logAnchor,
                  let from = logLines.firstIndex(where: { $0.seq == anchor }),
                  let to = logLines.firstIndex(where: { $0.seq == seq }) {
            selectedLogSeqs = Set(logLines[min(from, to)...max(from, to)].map(\.seq))
        } else {
            selectedLogSeqs = [seq]
            logAnchor = seq
        }
    }

    func clearLogView() {
        logsFollowing = true
        logLines = []
        selectedLogSeqs = []
        logAnchor = nil
    }

    func copySelectedLogLines() {
        let text = logLines
            .filter { selectedLogSeqs.contains($0.seq) }
            .map { line in
                let marker = line.stream == "stderr" ? "stderr " : ""
                return "\(TimeFormat.clock(ms: line.tsMs)) \(marker)\(line.text)"
            }
            .joined(separator: "\n")
        copyToPasteboard(text)
    }

    // MARK: Database

    func loadConnections() async {
        guard let root = workspace?.root else {
            connections = []
            return
        }
        do {
            connections = try await KestrlBridge.run { try dbConnections(workspaceRoot: root) }
        } catch {
            report(error)
        }
    }

    func setConnectionExpanded(_ name: String, expanded: Bool) {
        if expanded {
            expandedConnections.insert(name)
            if tablesByConnection[name] == nil {
                Task { await loadTables(connection: name) }
            }
        } else {
            expandedConnections.remove(name)
        }
    }

    func loadTables(connection: String) async {
        guard let root = workspace?.root else { return }
        let env = selectedEnvironment
        do {
            tablesByConnection[connection] = try await KestrlBridge.run {
                try dbTables(workspaceRoot: root, connection: connection, env: env)
            }
        } catch {
            report(error)
        }
    }

    /// Selecting a table fills in a preview query and shows its columns.
    func databaseSelectionChanged(from previous: DatabaseSelection?) async {
        if previous?.connection != selectedConnection {
            queryResult = nil
        }
        guard case .table(let table) = selectedDatabaseItem else {
            tableColumns = []
            return
        }
        sql = "SELECT * FROM \(table.qualifiedName) LIMIT 100"
        await loadColumns(table)
    }

    private func loadColumns(_ table: DatabaseTable) async {
        guard let root = workspace?.root else { return }
        let env = selectedEnvironment
        do {
            let columns = try await KestrlBridge.run {
                try dbDescribe(workspaceRoot: root, connection: table.connection, env: env, table: table.qualifiedName)
            }
            guard selectedDatabaseItem == .table(table) else { return }
            tableColumns = columns
        } catch {
            report(error)
        }
    }

    /// Closes the table inspector, keeping the connection selected.
    func closeTableInspector() {
        guard case .table(let table) = selectedDatabaseItem else { return }
        selectedDatabaseItem = .connection(table.connection)
    }

    /// Environment changes alter what the database returns, so the table lists and result are reloaded.
    func environmentChanged() async {
        queryResult = nil
        tablesByConnection = [:]
        for name in expandedConnections {
            await loadTables(connection: name)
        }
        if case .table(let table) = selectedDatabaseItem {
            await loadColumns(table)
        }
    }

    func runQuery() async {
        guard let root = workspace?.root, let connection = selectedConnection, !isRunningQuery else { return }
        let env = selectedEnvironment
        let statement = sql
        guard !statement.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        isRunningQuery = true
        defer { isRunningQuery = false }
        do {
            let record = try await KestrlBridge.run {
                try dbQuery(workspaceRoot: root, connection: connection, env: env, sql: statement)
            }
            queryResult = ShownQueryResult(record)
        } catch {
            report(error)
        }
    }

    private func resetDatabase() {
        connections = []
        tablesByConnection = [:]
        expandedConnections = []
        tableColumns = []
        queryResult = nil
        selectedDatabaseItem = nil
        sql = ""
    }

    // MARK: Copy

    func copyQueryResultJSON() {
        guard let result = queryResult else { return }
        copyToPasteboard(result.json())
    }

    func copyToPasteboard(_ text: String) {
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
    }

    func copyShownJSON() {
        guard let json = shownExchangeJSON() else { return }
        copyToPasteboard(json)
        justCopied = true
        Task {
            try? await Task.sleep(for: .seconds(1.5))
            justCopied = false
        }
    }

    private func shownExchangeJSON() -> String? {
        switch section {
        case .requests:
            guard let exchange = response?.exchange else { return nil }
            return ExchangeJSON.make(
                status: exchange.status,
                durationMs: exchange.durationMs,
                headers: exchange.headers,
                body: exchange.body,
                redacted: exchange.redacted,
                error: exchange.error
            )
        case .history:
            guard let http = eventDetail?.detail.http else { return nil }
            return ExchangeJSON.make(
                status: http.status,
                durationMs: http.durationMs,
                headers: http.resHeaders,
                body: http.resBody ?? "",
                redacted: http.redacted,
                error: http.error
            )
        case .capture, .environments, .secrets, .logs, .database:
            return nil
        }
    }

    private func prepareOptional(_ raw: String?) async -> PreparedBody? {
        guard let raw else { return nil }
        return await PreparedBody.prepare(raw)
    }
}
