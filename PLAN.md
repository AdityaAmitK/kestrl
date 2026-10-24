# Kestrl Plan

A fast, local-first workbench for backend development, built for developers and AI agents (Cursor, Claude Code, Codex). One tool instead of an API client, a traffic inspector, a database GUI and scattered terminal logs. Every feature has a **CLI** and a **GUI**, and both work on the same data. Agents use the CLI instead of curl.

## Non-goals

- **No accounts, no cloud sync, no hosted backend.** Kestrl stays local and light. Sharing happens through git (request files) and Copy JSON.
- No telemetry.
- No Postman-style workspace bloat. Postman collection import may come later.

## Glossary

Each term means one thing and is used the same way in code, the CLI and the docs.

### Actions

| Term | Meaning | Command |
|---|---|---|
| **Send** | You make one API request and get the response | `kestrl send` |
| **Run** | Send every request in a collection, in order | `kestrl run <dir>` |
| **Replay** | Send a past request again exactly as it was | `kestrl history replay <id>` |
| **Capture** | Kestrl observes traffic that something else makes | `kestrl capture …` |
| **Up** | Start a service through Kestrl so its logs are collected | `kestrl up` |
| **Attach** | Follow logs of a service Kestrl did not start (file or Docker) | `kestrl logs attach` |
| **Tail** | Read or follow a service's logs | `kestrl logs <service>` |
| **Query** | Run SQL against a database connection | `kestrl db query` |
| **Browse** | Look through database tables | GUI |

**Send vs capture:** *send* means you make the request; *capture* means you watch requests others make. The word "watch" is not used.

### Capture kinds

| Term | Observes | Command |
|---|---|---|
| **Device capture** | iOS Simulator and Android emulator traffic (HTTP and HTTPS) through Kestrl's proxy | `kestrl capture sim` / `kestrl capture android` |
| **Browser capture** | A separate browser window that Kestrl opens (Chrome DevTools Protocol) | `kestrl capture browser` |
| **Port capture** | Live plain-HTTP traffic on a local port, e.g. a server already running on 1337 | `kestrl capture port 1337` |

### Data

| Term | Meaning | Lives in |
|---|---|---|
| **Request file** | One saved request (`*.req.toml`) | Git |
| **Collection** | A folder of request files | Git |
| **Environment** | A named set of variables (`local`, `prod`) | Git |
| **Variable** | `{{api_url}}`, with a value per environment | Git |
| **Secret** | A variable whose value lives in the Keychain: `{{secret:name}}` | Keychain |
| **Profile** | A reusable add-on of headers and query params, optionally conditional (`admin`) | Git |
| **Connection** | A database, with host per environment and credentials in the Keychain | Git + Keychain |
| **Service** | A named process Kestrl runs or follows (`api`), with its port and PID | Memory |
| **Live buffer** | Temporary in-memory data: logs and captures | Memory |
| **History** | Requests and queries you ran, auto-pruned | SQLite |
| **Event** | One history entry | SQLite |
| **Saved** | A tagged or noted event, kept until deleted | SQLite |
| **Tag** | A label on an event (`#login-bug`) | SQLite |
| **Session** | A group of events from one working stretch | SQLite |
| **Actor** | Who did it: `me`, `cursor`, `claude-code`, `codex` | SQLite |
| **Timeline** | Requests, logs and queries from the same moment, shown together | View |

## Architecture

```
crates/
  kestrl-core     Library: requests, variables, profiles, secrets, history, logs, db, capture
  kestrl-cli      `kestrl` binary (clap) over kestrl-core
  kestrl-ffi      UniFFI bindings for the Swift app
app/              SwiftUI macOS app; links kestrl-ffi as an XCFramework
```

- **GUI:** SwiftUI over the Rust core via UniFFI. The core contains no GUI code.
- **Store:** SQLite at `~/Library/Application Support/kestrl/history.db` in WAL mode, so the app and CLI can use it at the same time. Project files live in the repo under `.kestrl/`.
- **Secrets:** macOS Keychain (service `kestrl`). Filled in only at send time and replaced with `••••••` before anything is printed or stored.
- **Output:** human-readable on a TTY; JSON when stdout is not a TTY or with `--json`.
- **Speed:** sync where possible, tokio only around network I/O, rustls, thin-LTO release builds, no daemon.

### FFI surface

`kestrl-ffi` exposes only these functions to the SwiftUI app. Each is a thin wrapper over `kestrl-core`, with no logic of its own. `scripts/build-xcframework.sh` builds `app/Frameworks/KestrlFFI.xcframework` and `app/Generated/kestrl_ffi.swift`.

| Area | Functions |
|---|---|
| Workspace | `open_workspace(path) -> WorkspaceInfo` (root, default env, envs, profiles, requests with method and name, retention settings) |
| Requests | `send_request_file(root, path, env, profiles, vars)`, `send_ad_hoc(root, method, url, env)` → `SentExchange`; recorded as actor `me`, source `app` |
| Requests | `read_request_file(root, path)` → `RequestDetail`, the file as written, variables unresolved |
| History | `list_events(filter)`, `show_event(id)`, `tag_event`, `untag_event`, `set_note`, `diff_events(a, b)` → JSON string |
| Environments | `resolved_env(root, env)` → `[KeyValue]`, secrets masked |
| Secrets | `set_secret(name, value)`, `delete_secret(name)`, `secret_status(root)` |

Errors are one `KestrlError` enum: `File`, `Request`, `Secret`, `History`, `InvalidInput`, each with a `message`. Logs, database and capture functions arrive with their phases.

## Project layout

```
.kestrl/
  kestrl.toml              # workspace config: default env, retention
  envs/local.toml
  envs/prod.toml
  profiles/admin.toml
  connections/main.toml
requests/
  auth/login.req.toml
```

`envs/local.toml`
```toml
[vars]
api_url = "http://localhost:3000"
token = "{{secret:local_token}}"
```

`requests/auth/login.req.toml`
```toml
name = "Login"
method = "POST"
url = "{{api_url}}/auth/login"
profiles = ["admin"]          # optional

[headers]
Authorization = "Bearer {{token}}"

[body]
type = "json"
content = '''{ "email": "a@b.c" }'''
```

`profiles/admin.toml`
```toml
[headers]
X-Admin-Token = "{{secret:admin_token}}"

[query]
debug = "1"

[when]                        # optional: apply automatically when every condition matches
env = ["local", "staging"]
url = "*/dev/*"
```

`connections/main.toml`
```toml
kind = "postgres"
allow_writes = ["local"]      # read-only everywhere else

[env.local]
url = "postgres://{{secret:db_local_user}}:{{secret:db_local_pass}}@localhost:5432/app"
```

**Resolution order:** `--var k=v` > environment > workspace config. Request-file headers and query params are the base values; profiles override them in this order: those listed in the request file, then `-p` flags, then profiles whose `when` matches. Later values override earlier ones. Secrets resolve last.

## Retention

| Data | Kept |
|---|---|
| Service logs | Memory only, rolling buffer (~10k lines per service), gone when the service stops |
| Captures | Memory only while capturing; save one to history explicitly |
| Sent requests and queries | Disk, pruned to the last 7 days or 500 events, whichever comes first |
| Saved events (tagged or noted) | Until deleted |
| Bodies | Cut to 1 MB when stored |

All limits are configurable in `.kestrl/kestrl.toml` under `[retention]`: `max_age_days`, `max_events`, `max_body_bytes`.

## History schema

The source of truth is `crates/kestrl-core/migrations/`.

- `sessions(id, name, actor, started_at, ended_at)`
- `events(id ULID, ts, kind http|db|capture, source app|cli|simulator|browser|port, actor, session_id, workspace, env, request_ref, correlation, summary, note)`
- `http_exchanges(event_id, method, url, headers, bodies, status, duration_ms, error, redacted)`, stored after masking
- `db_queries(event_id, connection, sql, rows, duration_ms, error)`
- `tags(event_id, tag)`
- `events_fts`: FTS5 index over summary, note, url and bodies

**Actor:** `--actor`, then `KESTRL_ACTOR`, then agent env vars (`CLAUDECODE`, `CODEX_*`, `CURSOR_*`), then the parent process chain, then `me`. Editable later.

## CLI surface

```
# Requests
kestrl send <file|METHOD URL> [-e env] [-p profile]... [--var k=v]... [--tag t] [--json]
kestrl run <dir> [-e env] [-p profile]...
kestrl envs [name]                         # list, or show resolved vars (secrets masked)
kestrl profiles [name]
kestrl secret set|rm|ls [name]             # `set` reads the value from stdin or a prompt

# History
kestrl history [query] [--by actor] [--tag t] [--since 1h] [--session id] [--limit n]
kestrl history show|replay <id>            # replay re-resolves from the request file if secrets were masked
kestrl history diff <a> <b>
kestrl history edit <id> [--set-actor a] [--note text]
kestrl tag <id> <tag> [--remove]
kestrl session start [name] | end

# Services and logs
kestrl up <service> -- <cmd...>
kestrl ps
kestrl logs <service> [-f] [--since 2m] [--grep re] [--json]
kestrl logs attach <service> --file <path> | --docker <container>

# Database
kestrl db query <connection> "<sql>" [-e env]
kestrl db tables <connection> [-e env] [table]
kestrl db connections

# Capture
kestrl capture sim|android|browser [--save]
kestrl capture browser [--browser chrome|arc|brave|edge|chromium|helium] [--browser-path <path>] [--url <start-url>] [--save] [--host <glob>]
kestrl capture port <port> [--save]
kestrl proxy trust --sim|--android          # install or stage the Kestrl root certificate
kestrl proxy ca                             # show the root certificate path and fingerprint
kestrl proxy restore                       # restore saved macOS proxy settings
```

The global `--json` option applies to database commands as well.

`send` JSON output: `{ id, status, ms, headers, body, redacted: ["local_token"] }`

## Phases

1. **Requests and history:** variables, environments, Keychain secrets and masking, profiles, `send`, `run`, history store and retention, `history`, `tag`, `session`.
2. **Services and logs:** `up`, `ps`, `logs` (tail, grep, follow), `attach`. In-memory buffers, with port detection per service.
3. **Database:** Postgres connections per environment, read-only by default, `db query` and `db tables`. Queries recorded in history.
4. **Capture:** an HTTPS interception proxy with its own root certificate, device capture (iOS Simulator, Android emulator), browser capture (CDP), and port capture (plain HTTP on loopback). The interception proxy binds to loopback only. The system proxy must be restored on exit, and again on the next start after a crash; restore retries retain state until every saved service is restored. Device capture commands are `kestrl capture sim|android [--port] [--save] [--host]`; browser capture launches a separate Chromium-family browser with an isolated temporary profile.
5. **GUI:** `kestrl-ffi` and the SwiftUI app: request editor, environment picker, history and timeline, log viewer, DB console, live capture, Copy JSON everywhere.
6. **Types (later):** schema inference from history, OpenAPI and JSON Schema import, validation, drift detection, type export.

**Later:** Postman collection import, physical-device capture over Wi-Fi.
