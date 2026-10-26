---
name: kestrl
description: Use the `kestrl` CLI instead of curl, ad-hoc psql, or scrolling terminal output when working on a backend that has a Kestrl workspace (a `.kestrl/` folder) or when the user mentions Kestrl. Covers sending API requests with environments and secrets, reading request history, reading service logs, querying Postgres read-only, and capturing traffic. Triggers on "send a request", "hit the endpoint", "check the logs", "what did the server return", "query the db", "replay that request", "capture traffic", or any API debugging in a repo containing `.kestrl/`.
---

# Kestrl

Kestrl is a local backend workbench with a CLI and a macOS app over the same data. Use it instead of `curl`, raw `psql`, and copy-pasted terminal logs: every request and query you run is recorded in shared history (attributed to you as the actor), secrets are injected at send time and masked in all output, and the user sees your work in the app.

Check it is available with `kestrl --version`. A workspace is any folder at or above the current directory containing `.kestrl/`.

## Rules

- Run `kestrl init [DIR]` only when the user asks to set up a workspace. It creates starter configuration and a health request; never initialize a workspace implicitly.
- **Prefer `kestrl send` over `curl`** for any request to the project's API, so it is recorded and secrets stay masked.
- **Never handle raw secrets.** Reference them as `{{secret:name}}`. If one is missing, tell the user to run `kestrl secret set <name>` themselves. Never run `secret set` with a value you invented or saw in output, and never read Kestrl's secrets file directly.
- **Output is JSON when piped**, so parse it (`| jq`) instead of scraping text. `redacted` lists which secrets were masked; `••••••` in output is a masked secret, not a bug.
- **Your actor is detected automatically** (Claude Code, Codex, Cursor). Set `--actor <name>` only if detection is wrong.
- **Database is read-only by default.** If a write fails with "is read-only in env", report it; do not edit `allow_writes` unless the user asks.
- **Ask before capture.** `capture sim` changes the macOS system proxy and `capture browser` opens a window; only run them when the user asks. If a capture is interrupted, `kestrl proxy restore` puts the proxy back.
- Do not start or stop the user's services with `kestrl up` unless asked; read logs from services already running.

## MCP tools and other machines

If the `kestrl` MCP server is configured (`kestrl mcp`), its tools (`send`, `history`, `ps`, `logs`, `logs_attach`, `stop`, `share_open`, `share_list`, `share_close`) are the same commands as below; use whichever is available. Every tool takes an optional `host`: an SSH host from the user's `~/.ssh/config` with Kestrl installed. Without MCP, run `ssh <host> -- sh -lc 'kestrl …'`. Work on the machine that has the service or files; follow a remote server's logs from this machine with `logs attach --ssh <host>`, and stop attaches you started with `kestrl stop <name>` when done.

## Requests

```sh
kestrl envs                                   # environments in this workspace
kestrl envs local                             # resolved variables (secrets masked)
kestrl envs set local api_url=http://localhost:4000   # only when the user asks to change config
kestrl envs new staging --from local          # duplicate an environment
kestrl send requests/auth/login.req.toml -e local
kestrl send GET '{{api_url}}/users' -e local  # ad hoc, variables still resolve
kestrl send POST '{{api_url}}/users' --json-body '{"name": "Ada"}' -H 'X-Tenant: acme'
kestrl send POST https://api.example.com/upload -d @payload.txt   # @path or @- (stdin)
TOKEN="$(cat ~/.config/example/token)" kestrl send GET https://api.example.com/me -H 'Authorization: Bearer {{env:TOKEN}}'
kestrl send requests/admin/stats.req.toml -p admin --var user_id=42
kestrl send requests/users/list.req.toml --tag login-bug
kestrl run requests/users -e local            # every request in a folder, in order
```

`send` JSON: `{ id, status, ms, headers, body, redacted, error }`. `id` is the history event id.

`kestrl send` works outside a workspace too, so use it instead of `curl` for any API call. `-H` overrides request and profile headers; `--json-body` sets `Content-Type`/`Accept` (the global `--json` only changes output). To use a token kept in a file, pass it through that one command's environment and reference it as `{{env:NAME}}`: it is masked in output and history and never appears in the process arguments. Fall back to `curl` only for what `kestrl send` cannot do (file downloads, streaming, multipart uploads), and say why.

Request files are plain TOML (`*.req.toml`): `method`, `url`, optional `profiles`, `[headers]`, `[query]`, `[body] type = "json"|"text"|"form"`. Environments are `.kestrl/envs/<env>.toml` with `[vars]`. Profiles (`.kestrl/profiles/<name>.toml`) add headers or query params, explicitly with `-p` or automatically via a `[when]` block. Precedence: `--var` > collection folder (`folder.toml` next to the request, nearest first) > environment > workspace; profiles override the request file's own headers and query. A folder's `default_env` is used when you don't pass `-e`.

## History

```sh
kestrl history --since 30m                    # newest first, default 20
kestrl history "users 500" --by me            # full-text search, filter by actor
kestrl history --tag login-bug
kestrl history show <id>                      # full request and response (ids accept unique prefixes)
kestrl history replay <id>                    # send it again, recorded as a new event
kestrl history diff <id-a> <id-b>             # status, timing, headers, JSON body paths
kestrl tag <id> login-bug
kestrl history edit <id> --note "fails without X-Tenant"
kestrl session start "fix login" / kestrl session end
```

Use history to see what the user (actor `me`) already tried before you start, and `diff` to compare a working and a failing call.

## Logs

```sh
kestrl ps                                     # services started with `kestrl up`, with ports
kestrl logs api --since 2m --grep 'error|panic'
kestrl logs api --limit 50
kestrl logs attach api --journal api.service --ssh my-server   # follow a server's logs over SSH
```

Logs are kept in memory only while the service runs. If `ps` shows nothing, the user's server was not started through Kestrl; ask them to restart it with `kestrl up <name> -- <their command>`, or attach without restarting anything: `kestrl logs attach <name> --file <path> | --docker <container> | --journal <unit>`, adding `--ssh <host>` for a remote machine. Attach blocks while it follows, so run it in the background and read with `kestrl logs <name>`; stop it by ending that process. Use only SSH hosts the user has configured; never change server permissions to read a journal.

## Share a port (only when asked)

```sh
kestrl share open 3000 --public --ttl 30m                      # temporary https://….trycloudflare.com URL
kestrl share open 3000 --protected --hostname app.example.com  # named hostname behind Cloudflare Access
kestrl share ls
kestrl share close <id>        # or --all
```

Sharing publishes a local web app, so only do it when the user asks, and only for the port they name. Access must be explicit: say which you used. `--protected` refuses unless the hostname has a Cloudflare Access application; do not fall back to `--public` without asking. Shares expire on their own; close them when done. Never share admin, database or log ports; logs stay private over SSH (`kestrl logs attach --ssh`).

## Database

```sh
kestrl db connections
kestrl db tables main -e local                # or: kestrl db tables main users
kestrl db query main "select id, email from users order by id desc limit 5" -e local
```

JSON: `{ columns, rows, row_count, ms }`. Queries are recorded in history.

## Capture (only when asked)

```sh
kestrl capture browser --url http://localhost:3000   # separate browser, isolated profile
kestrl capture sim --host api.example.com            # iOS Simulator via system proxy
kestrl capture port 4000                             # plain HTTP on a loopback port (needs one-time setup)
kestrl proxy restore                                 # if a capture was killed
```

Captures stream to stdout and are not stored unless `--save` is given.
