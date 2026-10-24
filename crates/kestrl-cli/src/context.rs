//! Per-invocation state shared by all commands.

use std::io::IsTerminal;

use kestrl_core::actor::Actor;

#[derive(Debug)]
pub struct Context {
    pub actor: Actor,
    /// Emit JSON instead of human-readable output.
    pub is_json: bool,
}

impl Context {
    pub fn new(actor_flag: Option<&str>, force_json: bool) -> Self {
        Self {
            actor: Actor::resolve(actor_flag),
            is_json: force_json || !std::io::stdout().is_terminal(),
        }
    }
}
