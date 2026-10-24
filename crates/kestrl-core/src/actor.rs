//! Who triggered an event: flag > KESTRL_ACTOR > env/parent-process detection.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    Me,
    Cursor,
    ClaudeCode,
    Codex,
    Other(String),
}

impl Actor {
    pub fn as_str(&self) -> &str {
        match self {
            Actor::Me => "me",
            Actor::Cursor => "cursor",
            Actor::ClaudeCode => "claude-code",
            Actor::Codex => "codex",
            Actor::Other(s) => s,
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "me" => Actor::Me,
            "cursor" => Actor::Cursor,
            "claude-code" | "claude" => Actor::ClaudeCode,
            "codex" => Actor::Codex,
            other => Actor::Other(other.to_owned()),
        }
    }

    /// Resolve the actor: explicit flag, then KESTRL_ACTOR, then env heuristics.
    pub fn resolve(flag: Option<&str>) -> Self {
        if let Some(f) = flag {
            return Self::parse(f);
        }
        if let Ok(v) = std::env::var("KESTRL_ACTOR") {
            return Self::parse(&v);
        }
        let has = |k: &str| std::env::var_os(k).is_some();
        if has("CLAUDECODE") {
            Actor::ClaudeCode
        } else if has("CODEX_SANDBOX") || has("CODEX_HOME") {
            Actor::Codex
        } else if has("CURSOR_TRACE_ID") || has("CURSOR_AGENT") {
            Actor::Cursor
        } else {
            // TODO: inspect parent process chain (sysctl KERN_PROC) for agent binaries.
            Actor::Me
        }
    }
}
