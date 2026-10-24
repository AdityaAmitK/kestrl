# Kestrl — agent conventions

Read `PLAN.md` first. Use its glossary terms exactly in code, CLI help and docs. These rules are binding for every contributor (human or agent).

## Workflow
- One feature = one branch (`feat/<name>`, `fix/<name>`) → PR to `main` → review → squash-merge.
- Keep PRs small and single-purpose. Never push to `main` directly.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo build` must pass.

## Code style (Rust) — follows the `rust-cli-agent-style` skill
- Simplest thing that works (ponytail): std before deps, no speculative abstractions.
- No `unwrap()`/`expect()` outside tests. `kestrl-core` uses `thiserror` (`error.rs`, `kestrl_core::Result`); `kestrl-cli` uses `anyhow` with `.context(...)`.
- `kestrl-core` never prints; only the CLI writes to stdout/stderr.
- One module per concern; every module starts with a `//!` doc. Private fields, `pub(crate)` for internals.
- Names: `is_/has_` booleans, `try_` fallible constructors, `*Options` config bundles, plural collections.
- Dependency versions only in the root `Cargo.toml` `[workspace.dependencies]`.
- Secrets: never log, store, or print a resolved secret; pass all output through `Redactor`.

## GUI (phase 5)
- SwiftUI over `kestrl-ffi` (UniFFI). Apply the `apple-design` and `frontend-design` skills: native controls, semantic colors, light/dark, keyboard shortcuts, accessibility first.

## Open source hygiene
- Never commit secrets or real credentials, including in fixtures and examples. CI runs gitleaks.
- Update `CHANGELOG.md` (Unreleased) and `PLAN.md` when behavior or the CLI surface changes.
