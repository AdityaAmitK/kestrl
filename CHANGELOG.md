# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Project plan, cargo workspace scaffold, and open-source project files.
- Memory-only service supervision and log buffers, service listing, log queries with tail, grep and follow, and file/Docker attach commands.
- `kestrl send`, `run`, `envs`, `profiles` and `secret`: request files, environments, variables, profiles, and Keychain secrets masked in all output.
- History store: events, sessions, tags and notes in SQLite, pruned to the last 7 days or 500 unsaved events; saved (tagged or noted) events are kept.
- `kestrl history [query] [--by actor] [--tag] [--since 30m|1h|2d] [--session] [--limit]`, `history show <id>`, `history edit <id> [--set-actor] [--note]`; ids may be unique prefixes.
- `kestrl tag <id> <tag> [--remove]` and `kestrl session start [name]` / `session end`.
- `kestrl send` and each request in `kestrl run` are recorded in history; `send --tag <tag>` tags the event, and the event id is printed (`"id"` in JSON).
- `kestrl history replay <id>`: sends a stored request again. If its secrets were masked, it is re-resolved from its request file with the event's environment.
- `kestrl history diff <a> <b>`: compares status, duration, response headers, and body (JSON paths or changed lines).
- `[retention]` in `.kestrl/kestrl.toml`: `max_age_days`, `max_events`, `max_body_bytes`.

### Changed
- `--since` accepts seconds (`30s`) as well as minutes, hours and days.
