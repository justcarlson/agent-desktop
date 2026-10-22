/// Whether a toolkit reports window extents in screen coordinates. GTK 4
/// reports them relative to the window on X11 and Wayland alike; a toolkit
/// that could not be read is treated as not reporting them, because a
/// position that is wrong would separate two windows that are not separate.
pub(crate) fn reports_global_position(name: Option<&str>, version: Option<&str>) -> bool {
    let Some(name) = name else { return false };
    if name.eq_ignore_ascii_case("gtk") {
        return version
            .and_then(|version| version.split('.').next())
            .and_then(|major| major.parse::<u32>().ok())
            .is_some_and(|major| major < 4);
    }
    ["qt", "chromium"]
        .iter()
        .any(|known| name.eq_ignore_ascii_case(known))
}

#[cfg(test)]
#[path = "toolkit_tests.rs"]
mod tests;
