# Kestrl

A fast, local-first workbench for backend development, built for developers and AI agents.

Kestrl puts an API client, request history, service logs, a Postgres console and traffic capture (iOS Simulator, Android emulator, browser, local ports) into one tool. Every feature has a CLI and a macOS app over the same data, so agents like Claude Code, Codex and Cursor can use `kestrl` instead of `curl`. Secrets stay in the macOS Keychain and are masked in all output. Plain-HTTP port capture is implemented for macOS loopback, but BPF access needs privilege. No safe one-time setup is available under the current restrictions on BPF permissions, groups, privileged helpers, and launch daemons. `kestrl capture port setup|status|uninstall` reports this blocker without changing system state; a separately approved signed helper is required before capture can start.

> **Status:** early development. See [PLAN.md](PLAN.md) for the design and roadmap.

## Install (from source)

Requires Rust (stable) and macOS.

```sh
git clone https://github.com/AdityaAmitK/kestrl
cd kestrl
cargo install --path crates/kestrl-cli
```

## Quick start

```sh
kestrl secret set local_token          # stored in the Keychain
kestrl send requests/auth/login.req.toml -e local
kestrl history --since 1h
```

Request files, environments and profiles are plain TOML under `.kestrl/` and `requests/`, so they can be committed and shared with git. See [PLAN.md](PLAN.md#project-layout).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: see [SECURITY.md](SECURITY.md).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
