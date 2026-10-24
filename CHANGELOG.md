# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
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
- macOS app: History is a full-width table (Time, Status, Method, Summary, Actor, Source, Tags, Duration) with event detail below it. Request files show their URL template in a top bar with a labeled Send (⌘↩); ad-hoc requests open from File ▸ New Request (⌘N). Responses sit below the request in a resizable split that remembers its position. The toolbar shows the environment name, and Copy JSON moved to the pane it copies.
- `history --json` events include `method`, `status` and `duration_ms` for HTTP events (null otherwise).
- `--since` accepts seconds (`30s`) as well as minutes, hours and days.
- Database queries use PostgreSQL result metadata to report affected rows, decode common date/time, UUID, JSON and numeric types, and support `sslmode=require`.
- Database query history uses workspace retention settings and records the active session.
- Sending and recording a request moved from the CLI into `kestrl-core::send`. CLI behavior is unchanged.
- A missing `.kestrl/envs/` or `.kestrl/profiles/` directory is an empty list, not an error.
