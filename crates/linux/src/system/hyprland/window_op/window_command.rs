//! The dispatch requests of each config dialect, all aimed by the target's
//! address. The selector matches only `address:0x` plus lower-case hex, and
//! an unmatched selector acts on no window (S9-31 to S9-35).

use super::config_dialect::ConfigDialect;

const SPECIAL_PREFIX: &str = "special:";

fn selector(address: u64) -> String {
    format!("address:{address:#x}")
}

/// Moves a floating window's top-left corner to `x`, `y`.
pub(crate) fn move_command(dialect: ConfigDialect, address: u64, x: i64, y: i64) -> String {
    match dialect {
        ConfigDialect::Lua => format!(
            "dispatch hl.dsp.window.move({{x={x},y={y},window=\"{}\"}})",
            selector(address)
        ),
        ConfigDialect::Legacy => {
            format!(
                "dispatch movewindowpixel exact {x} {y},{}",
                selector(address)
            )
        }
    }
}

/// Resizes a floating window to `width` by `height` around its centre.
pub(crate) fn resize_command(
    dialect: ConfigDialect,
    address: u64,
    width: i64,
    height: i64,
) -> String {
    match dialect {
        ConfigDialect::Lua => format!(
            "dispatch hl.dsp.window.resize({{x={width},y={height},window=\"{}\"}})",
            selector(address)
        ),
        ConfigDialect::Legacy => format!(
            "dispatch resizewindowpixel exact {width} {height},{}",
            selector(address)
        ),
    }
}

/// Moves a window to `workspace` without following it, so the user's view
/// and focus stay where they are.
pub(crate) fn workspace_command(dialect: ConfigDialect, address: u64, workspace: &str) -> String {
    let workspace = workspace_selector(workspace);
    match dialect {
        ConfigDialect::Lua => format!(
            "dispatch hl.dsp.window.move({{workspace=\"{workspace}\",follow=false,window=\"{}\"}})",
            selector(address)
        ),
        ConfigDialect::Legacy => format!(
            "dispatch movetoworkspacesilent {workspace},{}",
            selector(address)
        ),
    }
}

/// Maximizes a window. Only the Lua dialect aims it by address.
pub(crate) fn maximize_command(address: u64) -> String {
    format!(
        "dispatch hl.dsp.window.fullscreen({{mode=\"maximized\",action=\"set\",window=\"{}\"}})",
        selector(address)
    )
}

/// Clears a window's maximized or fullscreen state. Only the Lua dialect
/// aims it by address.
pub(crate) fn unset_fullscreen_command(address: u64) -> String {
    format!(
        "dispatch hl.dsp.window.fullscreen_state({{internal=0,client=0,action=\"set\",window=\"{}\"}})",
        selector(address)
    )
}

/// A workspace by the name `j/clients` reports: a special or numbered
/// workspace as it is, any other by `name:`.
fn workspace_selector(name: &str) -> String {
    if name.starts_with(SPECIAL_PREFIX)
        || (!name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
    {
        name.to_owned()
    } else {
        format!("name:{name}")
    }
}

#[cfg(test)]
#[path = "window_command_tests.rs"]
mod tests;
