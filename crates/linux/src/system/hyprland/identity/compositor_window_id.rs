//! Window ids for compositor toplevels that expose no accessibility frame:
//! `w-hypr-<compositor pid>-<compositor start time>-<identifier>`. The
//! identifier is the toplevel's `ext_foreign_toplevel_handle_v1`
//! identifier, which the compositor never reuses while it runs, so an id
//! names one toplevel of one compositor process and never another. The
//! prefix cannot be an accessibility window id, whose bus part is digits.

const PREFIX: &str = "w-hypr-";
const IDENTIFIER_LIMIT: usize = 64;

/// A compositor window id taken apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompositorWindowId {
    pub(crate) compositor_pid: u32,
    pub(crate) compositor_start: u64,
    pub(crate) identifier: String,
}

impl CompositorWindowId {
    /// Whether the compositor process `pid`, started at `start`, issued it.
    pub(crate) fn issued_by(&self, pid: u32, start: u64) -> bool {
        self.compositor_pid == pid && self.compositor_start == start
    }
}

/// The id of toplevel `identifier` of the compositor `pid` started at
/// `start`, or `None` when the identifier is not plain ASCII alphanumeric
/// of at most 64 characters.
pub(crate) fn encode(pid: u32, start: u64, identifier: &str) -> Option<String> {
    let plain = !identifier.is_empty()
        && identifier.len() <= IDENTIFIER_LIMIT
        && identifier.bytes().all(|byte| byte.is_ascii_alphanumeric());
    plain.then(|| format!("{PREFIX}{pid}-{start}-{identifier}"))
}

/// The parts of an id `encode` produced. Anything else is `None`.
pub(crate) fn decode(id: &str) -> Option<CompositorWindowId> {
    let mut parts = id.strip_prefix(PREFIX)?.splitn(3, '-');
    let compositor_pid = parts.next()?.parse().ok()?;
    let compositor_start = parts.next()?.parse().ok()?;
    let identifier = parts.next()?;
    (encode(compositor_pid, compositor_start, identifier).as_deref() == Some(id)).then(|| {
        CompositorWindowId {
            compositor_pid,
            compositor_start,
            identifier: identifier.to_owned(),
        }
    })
}

/// Whether `id` is exactly a form `encode` produces.
pub(crate) fn is_well_formed(id: &str) -> bool {
    decode(id).is_some()
}

/// Whether `id` has the compositor window id prefix, well formed or not.
pub(crate) fn is_compositor_id(id: &str) -> bool {
    id.starts_with(PREFIX)
}

#[cfg(test)]
#[path = "compositor_window_id_tests.rs"]
mod tests;
