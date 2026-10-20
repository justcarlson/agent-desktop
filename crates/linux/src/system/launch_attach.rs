const COMM_LEN: usize = 15;

/// Extra identity keys for launchers whose application runs under another
/// name than the one launched: the launcher of a terminal exits and the
/// terminal server it started owns the windows.
pub(super) const KEY_TABLE: &[(&str, &[&str])] = &[
    ("gnome-terminal", &["gnome-terminal-server"]),
    ("org.gnome.terminal", &["gnome-terminal-server"]),
];

/// A running application as the registry lists it, once per process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Running {
    pub(crate) pid: u32,
    pub(crate) instance: String,
    pub(crate) name: String,
    pub(crate) registry_name: Option<String>,
    pub(crate) comm: String,
}

/// What a launch does about the applications already running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    Spawn,
    Attach(Running),
    Forward(Running),
    Refuse(Vec<u32>),
    Ambiguous(Vec<u32>),
}

/// The lower-cased keys of a target with the table's extra keys, once each.
pub(super) fn identity_keys(raw: &[String]) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for key in raw.iter().map(|key| key.to_lowercase()) {
        let extra = KEY_TABLE
            .iter()
            .filter(|(known, _)| *known == key)
            .flat_map(|(_, more)| more.iter().map(|more| (*more).to_owned()));
        for candidate in std::iter::once(key.clone()).chain(extra) {
            if !keys.contains(&candidate) {
                keys.push(candidate);
            }
        }
    }
    keys
}

/// Whether a key names the application exactly: its listed name, its registry
/// name or its `comm`, which the kernel cuts at 15 characters. A substring
/// never matches.
pub(crate) fn matches(keys: &[String], app: &Running) -> bool {
    keys.iter().any(|key| {
        let same = |name: &str| name.eq_ignore_ascii_case(key);
        let cut =
            app.comm.len() == COMM_LEN && key.to_lowercase().starts_with(&app.comm.to_lowercase());
        same(&app.name) || app.registry_name.as_deref().is_some_and(same) || same(&app.comm) || cut
    })
}

/// Decides before anything is spawned. No match spawns. One match is attached
/// to, and spawns only when arguments have to reach it, in which case the
/// spawned process forwards them or starts a second instance. With
/// `attach_if_running` false, any match refuses; otherwise several matches
/// are ambiguous.
pub(crate) fn decide(
    keys: &[String],
    running: &[Running],
    attach_if_running: bool,
    has_arguments: bool,
) -> Decision {
    let found: Vec<&Running> = running.iter().filter(|app| matches(keys, app)).collect();
    let pids = || found.iter().map(|app| app.pid).collect::<Vec<_>>();
    match found.as_slice() {
        [] => Decision::Spawn,
        _ if !attach_if_running => Decision::Refuse(pids()),
        [one] if has_arguments => Decision::Forward((*one).clone()),
        [one] => Decision::Attach((*one).clone()),
        _ => Decision::Ambiguous(pids()),
    }
}

#[cfg(test)]
#[path = "launch_attach_tests.rs"]
mod tests;
