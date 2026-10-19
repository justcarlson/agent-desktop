use agent_desktop_core::KeyCombo;

pub(crate) fn is_blocked(_combo: &KeyCombo) -> bool {
    false
}

#[cfg(test)]
#[path = "blocked_combo_tests.rs"]
mod tests;
