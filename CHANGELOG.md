# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.3] - 2026-10-26

### Added
- Linux CLI: static x86_64 and aarch64 release binaries with checksums, and `install.sh` support for Linux. Requests, environments, secrets, history, services and logs, database queries, browser capture (Chromium-family browsers on `PATH`) and Android emulator capture work on Linux; iOS Simulator capture and port capture report that they are supported only on macOS. `kestrl ps` reads listening ports from `/proc` on Linux, so `lsof` is not required.

### Changed
- The service run directory is `run/` in the data directory on every platform (unchanged on macOS: `~/Library/Application Support/kestrl/run`).
- Secrets are stored in `secrets.toml` in Kestrl's data directory (owner-only, `KESTRL_DATA_DIR` overrides the location) instead of the macOS Keychain, so there are no Keychain access prompts and the store works without Apple frameworks. `{{secret:name}}`, masking, `kestrl secret set|rm|ls` and the app's Secrets section are unchanged, and values are still never shown. Existing Keychain items are not migrated or deleted; set the ones you still use again with `kestrl secret set <name>`.

## [0.1.2] - 2026-10-25

### Added
- Collections in the app: New Request, New Folder, Duplicate, Rename, Move to Trash and Reveal in Finder from the + menu and right-click; new requests are created as files and open immediately; empty folders show.
- Collection folder settings (`folder.toml`): variables for every request inside, per-environment overrides, and a default environment, used by the CLI and the app (the toolbar switches to a folder's default environment).
- Edit environments in the app: create (empty or as a copy), duplicate, rename, make default and delete environments; add, edit and remove variables with {{…}} highlighting and a resolved preview; add Keychain-backed secret variables. Edits are drafts until Save (⌘S), and switching away or quitting with unsaved edits asks first.
- `kestrl envs new|set|unset|rename|rm|default` to manage environments from the CLI. Edits keep comments and order in the TOML files and write atomically.
- Profiles section in the app: create, duplicate, rename and delete profiles; edit headers and query parameters as tables; choose whether a profile applies automatically by environment and URL pattern.
- Database connections in the app: create, edit, rename and delete connections with a URL per environment and per-environment write access.
- Secrets section: add new secrets with +, see unused stored secrets, replace or remove values.
- Query parameters, headers and form bodies edit as key/value tables, and request tabs show counts and open on the tab that has content.

### Changed
- Secret presence is read from Keychain attributes only, so listing secrets no longer triggers a Keychain access prompt for each secret. `kestrl secret ls` also lists stored secrets nothing references.

### Fixed
- Error banners no longer cover pane headers and the sidebar.
- Collapsing a request folder or a database connection no longer leaves the selected row on screen.

## [0.1.1] - 2026-10-25

### Added
- Workspace creation through `kestrl init [DIR]`, UniFFI and the app’s New Workspace command (⇧⌘N), with a local environment and starter health request. Opening an uninitialized folder offers to create a workspace. Existing files are never overwritten.
- macOS request fields highlight variables, secret references and undefined names, with fast hover cards and caret-anchored `{{` completions showing resolved values and secret presence.

### Changed
- Kestrl is now licensed under MIT only (previously MIT or Apache 2.0).
- Website adds agent-assisted CLI and skill setup, feature descriptions, and demo app and CLI views.
- Release builds of the macOS app are universal (Apple Silicon and Intel).

## [0.1.0] - 2026-10-25

### Added
- Static project website and documentation for installation, workspaces, CLI usage, agents and traffic capture.

### Fixed
- Capture host filters include subdomains for plain domains, correctly anchor wildcard suffixes, and omit excluded HTTPS tunnels from the capture list and history. The app clarifies when filters apply.
- Device capture takes an exclusive, cross-process lock before it touches any device proxy setting. A second `kestrl capture` or app capture fails with a clear error instead of restoring the first capture's proxy.
- `kestrl capture android` targets exactly one booted emulator, pins every `adb` call to it, and fails when none or several are running. Physical devices are no longer accepted.
- `kestrl capture android` saves the emulator's previous global `http_proxy` before changing it and restores that value on stop, instead of overwriting it with `:0`. A saved record survives a crash and is restored on the next device capture start; if the emulator is not running, the start fails and keeps the record.
- `kestrl capture browser` stops on Ctrl-C through a per-capture stop flag instead of a process-wide interrupt, so the app can start and stop captures repeatedly. A stop also ends the wait for a response body that the browser never sends.
- macOS app: the list column of each section starts at 240 pt (200–340 pt) instead of about 45% of the window; the sidebar is 180–240 pt. Log lines stay on one row, with horizontal scrolling, unless View ▸ Wrap log lines (⌥⌘W) is on.
- macOS app: auto-follow keeps log timestamps visible and only pauses on user scrolling; Clear resumes following new output.
- Browser capture removes abandoned `kestrl-browser-*` temporary profiles when no browser process is using them; device and browser capture lines display full URLs without default HTTP(S) ports.

### Added
- Agent skill (`skills/kestrl/SKILL.md`) for Claude Code, Codex and other agents.
- macOS app: a custom kestrel app icon for the Dock, Finder and app switcher.
- macOS request editor: editable method, URL, headers, query params, JSON/text/form bodies and profiles; send without saving, explicit Save/Save As, in-memory drafts, external-edit protection and confirmation before quitting with unsaved edits.
- macOS workbench: compact pane headers and empty states, searchable request collection, persistent top request bar, syntax-colored response bodies, and on-demand table columns.
- macOS app: Capture section captures from a browser, a loopback port, an iOS Simulator or an Android emulator. Pick a source and only its controls show; Start and Stop run one capture at a time. Exchanges appear live (newest first), with status, timing, headers, body, Copy JSON, and an opt-in Save to history. Device capture restores the device proxy on Stop, on quit and after an error. Port capture shows its one-time BPF setup command for you to run in Terminal; the app never runs it. Changing workspace stops the capture.
- `kestrl capture port <port>` plain-HTTP loopback packet capture with bounded TCP reassembly, HTTP/1 framing, host filtering, shared secret redaction and optional history recording. macOS BPF access uses a one-time administrator setup, `sudo sh scripts/kestrl-bpf.sh install --user <name>`, which creates a dedicated `kestrl_bpf` group and a root-owned boot-time LaunchDaemon. Membership lets that user read all interfaces through BPF, not only loopback. `kestrl capture port setup|status|uninstall` report the plan, the current access, and the removal command without changing system state.
- macOS app: Logs section streams a running service's log with a local-time gutter, stderr marking, pause-on-scroll, regex filter, and Copy and Clear.
- macOS app: a JSON log line with `level` and `message` shows a level chip, the message, then its other keys as `key=value`. The raw line shows on hover and is what Copy copies.
- macOS app: Database section lists connections and their tables, runs SQL with ⌘↩, shows results as a table with Copy JSON, and previews a table's columns.
- `kestrl capture browser`: launch an isolated Chromium-family browser and capture HTTP exchanges through Chrome DevTools Protocol, with optional host filters and history recording.
- Browser capture detects browser bundle executables from Info.plist in `/Applications` and `~/Applications`, supports Helium, and accepts `--browser-path` for other Chromium-family browsers.
- Device capture binds its interception proxy to loopback, restores system proxy settings after panics and crashes, skips disabled or device-less network services, and retries incomplete restores.
- Device capture can run outside a Kestrl workspace and prints local-time capture lines when stdout is redirected; Tokio runtime and session ordering are managed by core.
- `kestrl capture sim|android` live device capture, Kestrl root CA management, macOS system proxy recovery, and optional capture history.
- `kestrl proxy trust --sim|--android`, `kestrl proxy ca`, and `kestrl proxy restore`.
- Project plan, cargo workspace scaffold, and open-source project files.
- Memory-only service supervision and log buffers, service listing, log queries with tail, grep and follow, and file/Docker attach commands.
- `kestrl send`, `run`, `envs`, `profiles` and `secret`: request files, environments, variables, profiles, and Keychain secrets masked in all output.
- History store: events, sessions, tags and notes in SQLite, pruned to the last 7 days or 500 unsaved events; saved (tagged or noted) events are kept.
- `kestrl history [query] [--by actor] [--tag] [--since 30m|1h|2d] [--session] [--limit]`, `history show <id>`, `history edit <id> [--set-actor] [--note]`; ids may be unique prefixes.
- `kestrl tag <id> <tag> [--remove]` and `kestrl session start [name]` / `session end`.
- `kestrl db connections`, `db query` and `db tables` for environment-specific PostgreSQL connections, transaction-enforced read-only access, table browsing and query history searchable by SQL.
- `kestrl send` and each request in `kestrl run` are recorded in history; `send --tag <tag>` tags the event, and the event id is printed (`"id"` in JSON).
- `kestrl history replay <id>`: sends a stored request again. If its secrets were masked, it is re-resolved from its request file with the event's environment.
- `kestrl history diff <a> <b>`: compares status, duration, response headers, and body (JSON paths or changed lines).
- `[retention]` in `.kestrl/kestrl.toml`: `max_age_days`, `max_events`, `max_body_bytes`.
- `kestrl-ffi`: UniFFI bindings for the SwiftUI app. Workspace, request send, history, environment and secret functions, with one `KestrlError` type. Listed in `PLAN.md` under FFI surface.
- `kestrl-ffi`: service listing, log reading and cancellable live following, plus database connection, query and table browsing functions.
- `scripts/build-xcframework.sh`: builds `KestrlFFI.xcframework` (arm64 and x86_64) and the Swift bindings.
- macOS app (`app/`, SwiftUI over `kestrl-ffi`): browse and send request files, ad-hoc requests, response view with Copy JSON, history with tags, notes and compare, environments, secrets, and read-only settings. `scripts/build-app.sh` builds it.
- `kestrl-ffi::read_request_file` returns a request as written, with variables unresolved.

### Changed
- `kestrl capture sim|android|port` stop on Ctrl-C through the CLI's own handler. `kestrl-core` device capture takes a stop flag instead of installing signal handlers, so the app can stop it without a process-wide handler.
- macOS app capture lists keep at most 16 MB of text as well as 500 exchanges, and show how many exchanges were dropped.
- macOS app: History is a full-width table (Time, Status, Method, Summary, Actor, Source, Tags, Duration) with event detail below it. Request files show their URL template in a top bar with a labeled Send (⌘↩); ad-hoc requests open from File ▸ New Request (⌘N). Responses sit below the request in a resizable split that remembers its position. The toolbar shows the environment name, and Copy JSON moved to the pane it copies.
- `history --json` events include `method`, `status` and `duration_ms` for HTTP events (null otherwise).
- `--since` accepts seconds (`30s`) as well as minutes, hours and days.
- Database queries use PostgreSQL result metadata to report affected rows, decode common date/time, UUID, JSON and numeric types, and support `sslmode=require`.
- Database query history uses workspace retention settings and records the active session.
- Sending and recording a request moved from the CLI into `kestrl-core::send`. CLI behavior is unchanged.
- A missing `.kestrl/envs/` or `.kestrl/profiles/` directory is an empty list, not an error.
