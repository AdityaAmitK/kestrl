# Kestrl

A fast, local-first workbench for backend development, built for developers and AI agents.

Kestrl puts an API client, request history, service logs, a Postgres console and traffic capture (iOS Simulator, Android emulator, browser, local ports) into one tool. Every feature has a CLI and a macOS app over the same data, so agents like Claude Code, Codex and Cursor can use `kestrl` instead of `curl`. Secrets live in an owner-only file outside the workspace and are masked in all output. Plain-HTTP port capture reads macOS loopback through BPF, which needs a one-time administrator setup from a checkout: `sudo sh scripts/kestrl-bpf.sh install --user <name>`. It creates a dedicated `kestrl_bpf` group. Members of that group can read every interface through BPF, not only loopback. `kestrl capture port setup|status|uninstall` shows the plan, the current access, and the removal steps.

> **Status:** early development. See [PLAN.md](PLAN.md) for the design and roadmap.

## Install

Install the standalone CLI on macOS or Linux (x86_64, aarch64) with the shell installer:

```sh
curl -fsSL https://kestrl.adityakinjawadekar.com/install.sh | sh
```

On Linux the CLI is a static binary that runs on servers and headless machines. Everything except iOS Simulator capture and port capture works there; those two fail with a clear "supported only on macOS" error. Secrets, history and service logs live under `~/.local/share/kestrl`.

Or install the CLI or macOS app with Homebrew on macOS:

```sh
brew install adityaamitk/tap/kestrl
brew trust adityaamitk/tap   # once; Homebrew requires it for third-party casks
brew install --cask adityaamitk/tap/kestrl
```

The app is also available as `Kestrl-<version>.dmg` on [GitHub Releases](https://github.com/AdityaAmitK/kestrl/releases).

The app and CLI are ad-hoc signed, not notarized. On first launch, macOS may block the app; open **System Settings → Privacy & Security** and choose **Open Anyway**. You can also remove its quarantine attribute with `xattr -dr com.apple.quarantine /Applications/Kestrl.app`.

## Install from source

Requires Rust (stable) on macOS or Linux. The macOS app also needs Xcode and XcodeGen (`scripts/build-app.sh`).

```sh
git clone https://github.com/AdityaAmitK/kestrl
cd kestrl
cargo install --path crates/kestrl-cli
```

## Quick start

```sh
kestrl init                           # create a workspace in your project folder
kestrl send requests/health.req.toml    # start your API on localhost:3000 first
kestrl history --since 1h
```

Request files, environments and profiles are plain TOML under `.kestrl/` and `requests/`, so they can be committed and shared with git. See [PLAN.md](PLAN.md#project-layout).

## Agent skill

`skills/kestrl/SKILL.md` teaches Claude Code, Codex and other agents to use `kestrl` instead of `curl`. Install it with:

```sh
mkdir -p ~/.claude/skills/kestrl && cp skills/kestrl/SKILL.md ~/.claude/skills/kestrl/
mkdir -p ~/.codex/skills/kestrl  && cp skills/kestrl/SKILL.md ~/.codex/skills/kestrl/
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security issues: see [SECURITY.md](SECURITY.md).

## License

Licensed under the [MIT License](LICENSE).
