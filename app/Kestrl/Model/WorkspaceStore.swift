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

    var section: SidebarSection {
        didSet { defaults.set(section.rawValue, forKey: Self.sectionKey) }
    }
    var appearance: AppearanceOverride {
        didSet { defaults.set(appearance.rawValue, forKey: Self.appearanceKey) }
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
    var adHocMethod = "GET"
    var adHocURL = ""
    var isShowingOpenPanel = false
    var isShowingComparison = false
    var isSettingSecret = false

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        section = SidebarSection(rawValue: defaults.string(forKey: Self.sectionKey) ?? "") ?? .requests
        appearance = AppearanceOverride(rawValue: defaults.string(forKey: Self.appearanceKey) ?? "") ?? .system
    }

    var canSendAdHoc: Bool {
        workspace != nil && !isSending && !adHocURL.trimmingCharacters(in: .whitespaces).isEmpty
    }

    var hasShownExchange: Bool {
        switch section {
        case .requests: return response != nil
        case .history: return eventDetail?.detail.http != nil
        case .environments, .secrets: return false
        }
    }

    /// Changes whenever the history filter does, so the history view can restart its refresh loop.
    var historyQueryKey: String { "\(historyActor ?? "")|\(historyText)" }

    func dismissError() {
        errorMessage = nil
    }

    func report(_ error: Error) {
        errorMessage = ErrorText.message(for: error)
    }

    // MARK: Workspace

    func restoreLastWorkspace() async {
        guard workspace == nil, let path = defaults.string(forKey: Self.lastWorkspaceKey) else { return }
        await loadWorkspace(from: URL(fileURLWithPath: path))
    }

    func loadWorkspace(from url: URL) async {
        let path = url.path(percentEncoded: false)
        do {
            let info = try await KestrlBridge.run { try openWorkspace(path: path) }
            if workspace?.root != info.root {
                response = nil
                requestDetail = nil
                requestBody = nil
                eventDetail = nil
                selectedRequest = nil
                selectedEventIDs = []
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
        } catch {
            report(error)
        }
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
        case .environments:
            if inspectedEnvironment == nil { inspectedEnvironment = selectedEnvironment }
            await loadResolvedVariables()
        case .secrets:
            await loadSecrets()
        }
    }

    // MARK: Requests

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
            requestBody = await prepareOptional(detail.body)
        } catch {
            report(error)
        }
    }

    func sendSelectedRequest() async {
        guard let root = workspace?.root, let path = selectedRequest else { return }
        let env = selectedEnvironment
        await perform(requestPath: path) {
            try sendRequestFile(workspaceRoot: root, path: path, env: env, profiles: [], vars: [])
        }
    }

    func sendAdHocRequest() async {
        guard canSendAdHoc, let root = workspace?.root else { return }
        let method = adHocMethod
        let url = adHocURL.trimmingCharacters(in: .whitespaces)
        let env = selectedEnvironment
        await perform(requestPath: nil) {
            try sendAdHoc(workspaceRoot: root, method: method, url: url, env: env)
        }
    }

    private func perform(requestPath: String?, _ work: @escaping @Sendable () throws -> SentExchange) async {
        guard !isSending else { return }
        isSending = true
        defer { isSending = false }
        do {
            let exchange = try await KestrlBridge.run(work)
            let body = await PreparedBody.prepare(exchange.body)
            response = ShownResponse(requestPath: requestPath, exchange: exchange, body: body)
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

    // MARK: Copy

    func copyShownJSON() {
        guard let json = shownExchangeJSON() else { return }
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(json, forType: .string)
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
        case .environments, .secrets:
            return nil
        }
    }

    private func prepareOptional(_ raw: String?) async -> PreparedBody? {
        guard let raw else { return nil }
        return await PreparedBody.prepare(raw)
    }
}
