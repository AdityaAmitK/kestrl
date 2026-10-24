# Changelog

All notable changes are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Project plan, cargo workspace scaffold, and open-source project files.
- History store: events, sessions, tags and notes in SQLite, with pruning to the last 7 days or 500 unsaved events. Saved events (tagged or noted) are kept.
- `kestrl history` lists events with `--actor`, `--tag`, `--since 30m|1h|2d`, `--session` and `--limit`; `history show <id>` and `history edit <id> [--actor] [--note]`. Ids may be unique prefixes.
- `kestrl tag <id> <tag>` adds a tag (`--remove` deletes it) and `kestrl session start [name]` / `session end`.

### Changed
- `--actor` is a top-level flag (before the subcommand), so `history --actor` can filter by actor.
