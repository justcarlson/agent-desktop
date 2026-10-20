use super::app_alias::{AliasIndex, current_index};
use super::launch_attach::KEY_TABLE;
use super::process_facts::ProcessFacts;

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}

const DELETED_SUFFIX: &str = " (deleted)";

/// An executable's basename without the marker the kernel appends to the
/// link target of a binary that was removed or replaced while it runs, so an
/// application keeps its name across a package upgrade.
fn live_basename(basename: &str) -> &str {
    basename.strip_suffix(DELETED_SUFFIX).unwrap_or(basename)
}

/// The name an application is listed under: the executable's basename,
/// then `comm`, which the kernel cuts at 15 characters. It comes from `/proc`
/// alone, never from the accessibility root, so an application keeps one name
/// whether it answers or is stopped: core upgrades a stale reference to an
/// unresponsive application only when the listed name is unchanged.
pub(crate) fn app_name(facts: &ProcessFacts) -> String {
    non_blank(facts.exe_basename.as_deref())
        .map(live_basename)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&facts.comm)
        .to_owned()
}

/// The desktop entry names the process's executable answers to. The listed
/// name never follows them.
pub(crate) fn aliases_of(facts: &ProcessFacts) -> Vec<String> {
    aliases_in(&current_index(), facts)
}

/// As [`aliases_of`] over an index the caller already holds, for a caller that
/// asks about many processes.
pub(crate) fn aliases_in(index: &AliasIndex, facts: &ProcessFacts) -> Vec<String> {
    facts
        .exe_basename
        .as_deref()
        .map(live_basename)
        .map(|basename| index.names_for(basename))
        .unwrap_or_default()
}

/// Whether `query` names this application: exactly, ignoring case, its
/// listed name, the accessibility root's registry name while the root
/// answered, its `comm`, its executable basename or one of the desktop entry
/// names in `aliases`, including the launcher's known server names.
pub(crate) fn name_matches(
    query: &str,
    name: &str,
    registry_name: Option<&str>,
    facts: &ProcessFacts,
    aliases: &[String],
) -> bool {
    [
        Some(name),
        non_blank(registry_name),
        Some(facts.comm.as_str()),
        facts.exe_basename.as_deref().map(live_basename),
    ]
    .into_iter()
    .flatten()
    .chain(aliases.iter().map(String::as_str))
    .any(|candidate| {
        candidate.eq_ignore_ascii_case(query)
            || KEY_TABLE.iter().any(|(launcher, servers)| {
                launcher.eq_ignore_ascii_case(query)
                    && servers
                        .iter()
                        .any(|server| server.eq_ignore_ascii_case(candidate))
            })
    })
}

#[cfg(test)]
#[path = "app_name_tests.rs"]
mod tests;
