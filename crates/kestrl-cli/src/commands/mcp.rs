//! `kestrl mcp`: a Model Context Protocol server on stdio for agents. Each tool runs the same
//! `kestrl` command an agent would type, on this machine or, with `host`, on another machine over
//! SSH, so the adapter adds no behavior of its own.

use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};
use clap::Args;
use serde_json::{Value, json};

use crate::context::Context;

#[derive(Debug, Args)]
pub struct McpArgs {}

const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
/// Tool output beyond this is cut, so one call cannot flood the agent's context.
const MAX_OUTPUT_BYTES: usize = 100 * 1024;

const INSTRUCTIONS: &str = "Kestrl tools run the kestrl CLI. Use send instead of curl for HTTP and API calls; it records the request in Kestrl history and masks secrets. Pass `host` (an SSH host from the user's ~/.ssh/config) to run a tool on another machine; omit it for this machine. Read logs with logs; follow a file, Docker container or systemd journal with logs_attach, and stop it with stop when done. Logs stay private: never share a log or admin port. Open a share only when the user asks, only for the port they name, and state whether it is public or protected.";

/// Serve MCP requests from stdin until it closes.
pub fn run(ctx: &Context, _args: McpArgs) -> anyhow::Result<()> {
    let mut stdout = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            reply(
                &mut stdout,
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}}),
            )?;
            continue;
        };
        // Notifications, such as `notifications/initialized`, get no reply.
        let Some(id) = message.get("id").cloned() else {
            continue;
        };
        let method = message["method"].as_str().unwrap_or_default();
        let result = match method {
            "initialize" => Ok(initialize(&message["params"])),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => Ok(call(ctx, &message["params"])),
            _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
        };
        let response = match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(error) => json!({"jsonrpc": "2.0", "id": id, "error": error}),
        };
        reply(&mut stdout, response)?;
    }
    Ok(())
}

fn reply(stdout: &mut impl Write, message: Value) -> anyhow::Result<()> {
    writeln!(stdout, "{message}")?;
    stdout.flush()?;
    Ok(())
}

fn initialize(params: &Value) -> Value {
    let requested = params["protocolVersion"].as_str().unwrap_or_default();
    let version = PROTOCOL_VERSIONS
        .into_iter()
        .find(|version| *version == requested)
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "kestrl", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Vec<Value> {
    let host = json!({
        "type": "string",
        "description": "SSH host (from ~/.ssh/config) to run on; omit for this machine"
    });
    let tool = |name: &str, description: &str, mut properties: Value, required: &[&str]| {
        properties["host"] = host.clone();
        json!({
            "name": name,
            "description": description,
            "inputSchema": { "type": "object", "properties": properties, "required": required },
        })
    };
    vec![
        tool(
            "send",
            "Send one HTTP request with kestrl send and return status, headers and body. Recorded in Kestrl history. Values may use {{var}}, {{secret:name}} and {{env:NAME}} (masked in output).",
            json!({
                "method": { "type": "string", "description": "HTTP method, e.g. GET" },
                "url": { "type": "string" },
                "headers": { "type": "array", "items": { "type": "string" }, "description": "Headers as `Name: value`" },
                "body": { "type": "string", "description": "Text body; `@path` reads a file on the target machine" },
                "json_body": { "type": "string", "description": "JSON body; sets Content-Type and Accept" },
                "env": { "type": "string", "description": "Kestrl environment of the current workspace" }
            }),
            &["method", "url"],
        ),
        tool(
            "history",
            "Search recent Kestrl history (requests and queries), newest first.",
            json!({
                "query": { "type": "string", "description": "Full-text search" },
                "since": { "type": "string", "description": "e.g. 30m, 2h" },
                "limit": { "type": "integer" }
            }),
            &[],
        ),
        tool(
            "ps",
            "List services Kestrl runs or follows (kestrl up, logs attach) with their ports.",
            json!({}),
            &[],
        ),
        tool(
            "logs",
            "Read a service's live log buffer.",
            json!({
                "service": { "type": "string" },
                "since": { "type": "string", "description": "e.g. 2m, 1h" },
                "grep": { "type": "string", "description": "Regular expression filter" },
                "limit": { "type": "integer", "description": "Last n lines (default 200)" }
            }),
            &["service"],
        ),
        tool(
            "logs_attach",
            "Start following a log file, Docker container or systemd unit journal as a service, in the background; read it with logs and end it with stop. Only reads; nothing is restarted. Give exactly one of file, docker or journal; ssh follows it on that host over SSH.",
            json!({
                "service": { "type": "string", "description": "Name to show in ps and read with logs" },
                "file": { "type": "string" },
                "docker": { "type": "string", "description": "Container name" },
                "journal": { "type": "string", "description": "systemd unit" },
                "ssh": { "type": "string", "description": "SSH host whose logs to follow" }
            }),
            &["service"],
        ),
        tool(
            "stop",
            "Stop a service started with kestrl up or logs_attach. Only stop services you started, unless the user asks.",
            json!({ "service": { "type": "string" } }),
            &["service"],
        ),
        tool(
            "share_open",
            "Expose one local web port at a URL through a Cloudflare tunnel, for a limited time. Only when the user asks. access must be stated: public (anyone with the URL) or protected (Cloudflare Access; needs hostname with an Access application).",
            json!({
                "port": { "type": "integer" },
                "access": { "type": "string", "enum": ["public", "protected"] },
                "hostname": { "type": "string", "description": "Named hostname; omit for a temporary trycloudflare.com URL" },
                "tunnel": { "type": "string", "description": "Named tunnel for hostname (default kestrl)" },
                "https": { "type": "boolean", "description": "The local server speaks HTTPS" },
                "ttl": { "type": "string", "description": "Lifetime such as 30m or 2h (default 1h, at most 24h)" }
            }),
            &["port", "access"],
        ),
        tool("share_list", "List open shares.", json!({}), &[]),
        tool(
            "share_close",
            "Close a share by id.",
            json!({ "id": { "type": "string" } }),
            &["id"],
        ),
    ]
}

/// How a tool's command runs.
enum Run {
    /// Wait for it and return its output.
    Wait(Vec<String>),
    /// Start it in the background and confirm that service `name` appears in `ps`.
    Detach { args: Vec<String>, name: String },
}

fn call(ctx: &Context, params: &Value) -> Value {
    let arguments = &params["arguments"];
    let host = text(arguments, "host");
    let outcome = command(params["name"].as_str().unwrap_or_default(), arguments)
        .and_then(|run| execute(ctx, host.as_deref(), run));
    let (output, is_error) = match outcome {
        Ok(output) => (output, false),
        Err(error) => (format!("{error:#}"), true),
    };
    json!({ "content": [{ "type": "text", "text": output }], "isError": is_error })
}

fn command(name: &str, arguments: &Value) -> anyhow::Result<Run> {
    let mut args: Vec<String> = Vec::new();
    let mut push = |values: &[&str]| args.extend(values.iter().map(|value| (*value).to_owned()));
    match name {
        "send" => {
            push(&[
                "send",
                &required(arguments, "method")?,
                &required(arguments, "url")?,
            ]);
            for header in arguments["headers"].as_array().into_iter().flatten() {
                push(&["-H", header.as_str().unwrap_or_default()]);
            }
            optional(&mut push, arguments, "body", "-d");
            optional(&mut push, arguments, "json_body", "--json-body");
            optional(&mut push, arguments, "env", "-e");
        }
        "history" => {
            push(&["history"]);
            if let Some(query) = text(arguments, "query") {
                push(&[&query]);
            }
            optional(&mut push, arguments, "since", "--since");
            push(&[
                "--limit",
                &number(arguments, "limit").unwrap_or(20).to_string(),
            ]);
        }
        "ps" => push(&["ps"]),
        "logs" => {
            push(&["logs", &required(arguments, "service")?]);
            optional(&mut push, arguments, "since", "--since");
            optional(&mut push, arguments, "grep", "--grep");
            push(&[
                "--limit",
                &number(arguments, "limit").unwrap_or(200).to_string(),
            ]);
        }
        "logs_attach" => {
            let service = required(arguments, "service")?;
            push(&["logs", "attach", &service]);
            let sources: Vec<_> = ["file", "docker", "journal"]
                .into_iter()
                .filter_map(|kind| text(arguments, kind).map(|value| (kind, value)))
                .collect();
            let [(kind, value)] = sources.as_slice() else {
                bail!("give exactly one of file, docker or journal");
            };
            push(&[&format!("--{kind}"), value]);
            optional(&mut push, arguments, "ssh", "--ssh");
            return Ok(Run::Detach {
                args,
                name: service,
            });
        }
        "stop" => push(&["stop", &required(arguments, "service")?]),
        "share_open" => {
            let port = number(arguments, "port").context("port is required")?;
            let access = match required(arguments, "access")?.as_str() {
                "public" => "--public",
                "protected" => "--protected",
                other => bail!("access must be public or protected, not {other:?}"),
            };
            push(&["share", "open", &port.to_string(), access]);
            optional(&mut push, arguments, "hostname", "--hostname");
            optional(&mut push, arguments, "tunnel", "--tunnel");
            optional(&mut push, arguments, "ttl", "--ttl");
            if arguments["https"].as_bool() == Some(true) {
                push(&["--https"]);
            }
        }
        "share_list" => push(&["share", "ls"]),
        "share_close" => push(&["share", "close", &required(arguments, "id")?]),
        other => bail!("unknown tool {other:?}"),
    }
    Ok(Run::Wait(args))
}

fn text(arguments: &Value, key: &str) -> Option<String> {
    arguments[key]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn number(arguments: &Value, key: &str) -> Option<u64> {
    arguments[key].as_u64()
}

fn required(arguments: &Value, key: &str) -> anyhow::Result<String> {
    text(arguments, key).with_context(|| format!("{key} is required"))
}

fn optional(push: &mut impl FnMut(&[&str]), arguments: &Value, key: &str, flag: &str) {
    if let Some(value) = text(arguments, key) {
        push(&[flag, &value]);
    }
}

fn execute(ctx: &Context, host: Option<&str>, run: Run) -> anyhow::Result<String> {
    // Remote commands do not see this agent's environment, so pass the actor on.
    let global = [
        "--json".to_owned(),
        "--actor".to_owned(),
        ctx.actor.as_str().to_owned(),
    ];
    match run {
        Run::Wait(args) => output(invocation(host, &global, &args, false)?),
        Run::Detach { args, name } => {
            let mut child = invocation(host, &global, &args, true)?;
            child
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if host.is_none() {
                // Its own process group, so it outlives this server and is stopped with `stop`.
                use std::os::unix::process::CommandExt;
                child.process_group(0);
            }
            let mut started = child.spawn().context("could not start kestrl")?;
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let services = output(invocation(host, &global, &["ps".to_owned()], false)?)?;
                let is_listed = serde_json::from_str::<Value>(&services)
                    .ok()
                    .and_then(|value| value.as_array().cloned())
                    .is_some_and(|list| list.iter().any(|service| service["name"] == name));
                if is_listed {
                    return Ok(format!(
                        "Following logs as service {name:?}. Read them with the logs tool and end with stop."
                    ));
                }
                if host.is_none() && started.try_wait()?.is_some() || Instant::now() >= deadline {
                    bail!(
                        "logs attach for {name:?} did not start; run `kestrl logs attach` in a terminal to see why"
                    );
                }
            }
        }
    }
}

/// The command that runs `kestrl <global> <args>` here, or on `host` through a login shell so
/// `~/.local/bin` is on PATH. `is_detached` backgrounds the remote command.
fn invocation(
    host: Option<&str>,
    global: &[String],
    args: &[String],
    is_detached: bool,
) -> anyhow::Result<Command> {
    let Some(host) = host else {
        let mut command = Command::new(std::env::current_exe()?);
        command.args(global).args(args);
        return Ok(command);
    };
    if host.starts_with('-') {
        bail!("invalid SSH host {host:?}");
    }
    let line: Vec<String> = std::iter::once("kestrl".to_owned())
        .chain(global.iter().chain(args).map(|arg| quote(arg)))
        .collect();
    let script = if is_detached {
        format!("nohup {} >/dev/null 2>&1 </dev/null &", line.join(" "))
    } else {
        format!("exec {}", line.join(" "))
    };
    let mut command = Command::new("ssh");
    command.args([
        "-o",
        "BatchMode=yes",
        "-o",
        "ConnectTimeout=10",
        host,
        "--",
        &format!("sh -lc {}", quote(&script)),
    ]);
    Ok(command)
}

fn output(mut command: Command) -> anyhow::Result<String> {
    let result = command
        .stdin(Stdio::null())
        .output()
        .context("could not run kestrl")?;
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);
    if !result.status.success() {
        bail!(
            "{}",
            truncate(stderr.trim().to_owned() + "\n" + stdout.trim()).trim()
        );
    }
    Ok(truncate(if stdout.trim().is_empty() {
        stderr.trim().to_owned()
    } else {
        stdout.into_owned()
    }))
}

fn truncate(mut text: String) -> String {
    if text.len() > MAX_OUTPUT_BYTES {
        let mut end = MAX_OUTPUT_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\n… output truncated; narrow the request (limit, since, grep)");
    }
    text
}

fn quote(argument: &str) -> String {
    format!("'{}'", argument.replace('\'', r"'\''"))
}
