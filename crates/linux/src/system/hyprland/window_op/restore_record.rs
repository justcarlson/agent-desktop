//! The geometry a maximize records on the window as a Hyprland tag, so a
//! later restore can put the window back. Hyprland's own floating geometry
//! goes stale when an output moves under a floating window (L46-3), and a
//! one-shot command keeps no state between a maximize and a restore, so
//! the record rides on the window itself (L46-2).

/// The prefix of every restore tag.
pub(crate) const RESTORE_TAG_PREFIX: &str = "agent-desktop-restore:";

/// The window's top-left corner relative to its monitor's origin, and its
/// size, when it was maximized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestoreRecord {
    pub(crate) offset: (i64, i64),
    pub(crate) size: (i64, i64),
}

impl RestoreRecord {
    /// The tag that carries this record.
    pub(crate) fn tag(&self) -> String {
        format!(
            "{RESTORE_TAG_PREFIX}{},{},{},{}",
            self.offset.0, self.offset.1, self.size.0, self.size.1
        )
    }

    /// The record a restore tag carries, or `None` for any other tag or a
    /// size that is not positive.
    pub(crate) fn parse(tag: &str) -> Option<Self> {
        let fields: Vec<i64> = tag
            .strip_prefix(RESTORE_TAG_PREFIX)?
            .split(',')
            .map(|field| field.parse().ok())
            .collect::<Option<_>>()?;
        match fields.as_slice() {
            [x, y, width, height] if *width > 0 && *height > 0 => Some(Self {
                offset: (*x, *y),
                size: (*width, *height),
            }),
            _ => None,
        }
    }
}

/// Every restore tag among `tags`, readable or not.
pub(crate) fn restore_tags(tags: &[String]) -> Vec<&str> {
    tags.iter()
        .map(String::as_str)
        .filter(|tag| tag.starts_with(RESTORE_TAG_PREFIX))
        .collect()
}

#[cfg(test)]
#[path = "restore_record_tests.rs"]
mod tests;
