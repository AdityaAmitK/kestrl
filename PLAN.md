# Kestrl Plan

A fast local workbench for backend development, built for me and for AI agents (Cursor, Claude Code, Codex). It covers what I use Postman/Bruno, Proxyman, TablePlus and terminal logs for today. Every feature has a GUI and a CLI, and both work on the same data.

## Architecture

```
crates/
  kestrl-core   Rust library: requests, envs, secrets, history, (later) logs, db, proxy
  kestrl-cli    `kestrl` binary (clap) over kestrl-core
  kestrl-ffi    (phase 5) UniFFI bindings for the Swift app
app/            (phase 5) SwiftUI macOS app; links kestrl-ffi as an XCFramework
```

- **GUI: SwiftUI over the Rust core via UniFFI** (decided). The core has no GUI code; everything the app needs goes through `kestrl-ffi`.
- **One store:** SQLite at `~/Library/Application Support/kestrl/history.db` (WAL, so the app and CLI can use it at the same time). Project files (requests, envs) live in the repo under `.kestrl/`.
- **Secrets:** macOS Keychain (service `kestrl`). Written as `{{secret:name}}`, filled in only at send time, and replaced with `••••••` before anything is printed or saved.
- **Output:** human-readable on a TTY. JSON when stdout is not a TTY, or when `--json` is passed.
- **Speed:** sync code where possible, with tokio only around reqwest. rustls, a thin-LTO release build, and no daemon in phase 1.

## Project layout (git-friendly)

```
.kestrl/
  kestrl.toml            # workspace config (default env, name)
  envs/local.toml
  envs/staging.toml
requests/
  auth/login.req.toml
```

`envs/local.toml`
```toml
[vars]
base_url = "http://localhost:3000"
token = "{{secret:local_token}}"
```

`login.req.toml`
```toml
name = "Login"
method = "POST"
url = "{{base_url}}/auth/login"

[headers]
Authorization = "Bearer {{token}}"

[body]
type = "json"
content = '''{ "email": "a@b.c" }'''
```

Variables resolve in this order: CLI `--var k=v` > env file > workspace config. Secrets resolve last.

## History schema

The source of truth is `crates/kestrl-core/migrations/0001_init.sql`.

- `sessions(id, name, actor, started_at, ended_at)`: groups the events from one working session.
- `events(id ULID, ts, kind http|log|db, source app|cli|simulator, actor, session_id, workspace, env, request_ref, correlation, summary, note)`: one row per recorded thing. Request, log and DB events link to each other through `correlation`.
- `http_exchanges(event_id, method, url, headers, bodies, status, duration_ms, error, redacted)`: stored *after* secrets are masked.
- `tags(event_id, tag)`
- `events_fts`: FTS5 index over summary, note, url and bodies.
- Phases 2 and 3 add `log_lines` and `db_queries` tables, also keyed by `event_id`.

**Actor:** `--actor`, then `KESTRL_ACTOR`, then env-var detection (`CLAUDECODE`, `CODEX_*`, `CURSOR_*`), then the parent process chain, then `me`. The actor can be changed later with `kestrl history edit`.

## CLI surface (phase 1)

```
kestrl send <file|METHOD URL> [-e env] [--var k=v] [--actor a] [--tag t] [--json]
kestrl run <dir> [-e env]                 # run every request in the folder, print a summary
kestrl envs [name]                        # list envs / show resolved vars (secrets masked)
kestrl secret set|rm|ls <name>            # `set` reads the value from stdin/prompt
kestrl history [query] [--actor a] [--tag t] [--since 1h] [--session id] [--limit n]
kestrl history show <id> | replay <id> | diff <a> <b> | edit <id> --actor/--note
kestrl tag <id> <#tag> [--note text]
kestrl session start [name] | end
```

`send` JSON output:
`{ id, status, ms, headers, body, redacted: ["local_token"] }`

## Phases

1. **Core + CLI:** workspace, history store and migrations, envs, request files, Keychain secrets and masking, `send/run/envs/secret/history/tag/session`.
2. **Logs:** `kestrl up -- <cmd>` captures stdout and stderr into a ring buffer plus SQLite; `kestrl logs <name> --since --grep --json`.
3. **DB:** Postgres connections per env, with creds in Keychain. `kestrl db query`, read-only by default, writes only when the env opts in. Queries are recorded in history.
4. **Proxy:** HTTP/HTTPS MITM for the iOS Simulator and Android emulator, with cert setup helpers. Captured traffic goes into history.
5. **GUI:** `kestrl-ffi` plus the SwiftUI app: request editor, history and timeline, log viewer, DB console.
6. **Types:** schema inference from history, OpenAPI/JSON Schema import, validation, drift detection, codegen.
