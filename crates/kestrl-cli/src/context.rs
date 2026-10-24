//! Per-invocation state shared by all commands.

use std::io::IsTerminal;

use kestrl_core::actor::Actor;
use time::UtcOffset;

#[derive(Debug)]
pub struct Context {
    pub actor: Actor,
    /// Emit JSON instead of human-readable output.
    pub is_json: bool,
    pub utc_offset: UtcOffset,
}

impl Context {
    pub fn new(actor_flag: Option<&str>, force_json: bool) -> Self {
        Self {
            actor: Actor::resolve(actor_flag),
            is_json: force_json || !std::io::stdout().is_terminal(),
            utc_offset: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }
}
