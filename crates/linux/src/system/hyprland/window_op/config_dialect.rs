//! Which config manager the bound instance runs. Hyprland 0.57.0 runs Lua
//! only; 0.56.2 runs Lua or the legacy hyprlang config, and each accepts a
//! different dispatcher syntax (S9-1 to S9-3).

/// The request that tells the two config managers apart.
pub(crate) const DIALECT_PROBE: &str = "eval return 1";

const LEGACY_REPLY: &str = "eval is only supported with the lua config manager";

/// The dispatcher syntax the bound instance accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigDialect {
    Lua,
    Legacy,
}

/// The dialect a reply to `DIALECT_PROBE` names, or `None` for any other
/// reply.
pub(crate) fn dialect_of(reply: &str) -> Option<ConfigDialect> {
    let reply = reply.trim();
    if reply == "ok" {
        Some(ConfigDialect::Lua)
    } else if reply.contains(LEGACY_REPLY) {
        Some(ConfigDialect::Legacy)
    } else {
        None
    }
}

#[cfg(test)]
#[path = "config_dialect_tests.rs"]
mod tests;
