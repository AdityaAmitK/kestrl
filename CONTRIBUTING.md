# Contributing to Kestrl

Thanks for your interest. Read [PLAN.md](PLAN.md) for the design and glossary, and [AGENTS.md](AGENTS.md) for the code conventions. Both apply to every contribution.

## Workflow

1. Open an issue first for anything larger than a small fix.
2. Branch from `main`: `feat/<name>` or `fix/<name>`.
3. Keep the PR small and focused on one change.
4. Before pushing, run:
   ```sh
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo build
   ```
5. Open a PR using the template. CI must pass before merge. PRs are squash-merged.

## Rules that matter most

- Never commit secrets, tokens or real credentials, including in examples and fixtures. Use `{{secret:name}}` placeholders.
- No `unwrap()` or `expect()` outside tests.
- The `kestrl-core` crate never prints. Only the CLI writes output.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
