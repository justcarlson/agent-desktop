use super::process_facts::ProcessFacts;

fn non_blank(text: Option<&str>) -> Option<&str> {
    text.filter(|text| !text.trim().is_empty())
}

/// The name an application is listed under: the accessibility root's `Name`
/// when it answered with a non-blank one, otherwise what `/proc` says. The
/// executable's basename comes before `comm`, which the kernel cuts at 15
/// characters. An application that did not answer is still named, so a
/// stopped process stays in the inventory.
pub(crate) fn app_name(root_name: Option<&str>, facts: &ProcessFacts) -> String {
    non_blank(root_name)
        .or(non_blank(facts.exe_basename.as_deref()))
        .unwrap_or(&facts.comm)
        .to_owned()
}

/// Whether `query` names this application: exactly, ignoring case, its
/// listed name, its `comm` or its executable basename.
pub(crate) fn name_matches(query: &str, name: &str, facts: &ProcessFacts) -> bool {
    [
        Some(name),
        Some(facts.comm.as_str()),
        facts.exe_basename.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|candidate| candidate.eq_ignore_ascii_case(query))
}

#[cfg(test)]
#[path = "app_name_tests.rs"]
mod tests;
