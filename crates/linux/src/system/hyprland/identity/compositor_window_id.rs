//! Window ids for compositor toplevels that expose no accessibility frame:
//! `w-hypr-<compositor pid>-<compositor start time>-<identifier>`. The
//! identifier is the toplevel's `ext_foreign_toplevel_handle_v1`
//! identifier, which the compositor never reuses while it runs, so an id
//! names one toplevel of one compositor process and never another. The
//! prefix cannot be an accessibility window id, whose bus part is digits.

const PREFIX: &str = "w-hypr-";
const IDENTIFIER_LIMIT: usize = 64;

/// The id of toplevel `identifier` of the compositor `pid` started at
/// `start`, or `None` when the identifier is not plain ASCII alphanumeric
/// of at most 64 characters.
pub(crate) fn encode(pid: u32, start: u64, identifier: &str) -> Option<String> {
    let plain = !identifier.is_empty()
        && identifier.len() <= IDENTIFIER_LIMIT
        && identifier.bytes().all(|byte| byte.is_ascii_alphanumeric());
    plain.then(|| format!("{PREFIX}{pid}-{start}-{identifier}"))
}

/// Whether `id` is exactly a form `encode` produces.
pub(crate) fn is_well_formed(id: &str) -> bool {
    let Some(rest) = id.strip_prefix(PREFIX) else {
        return false;
    };
    let mut parts = rest.splitn(3, '-');
    let (Some(pid), Some(start), Some(identifier)) = (parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    let (Ok(pid), Ok(start)) = (pid.parse::<u32>(), start.parse::<u64>()) else {
        return false;
    };
    encode(pid, start, identifier).as_deref() == Some(id)
}

/// Whether `id` has the compositor window id prefix, well formed or not.
pub(crate) fn is_compositor_id(id: &str) -> bool {
    id.starts_with(PREFIX)
}

#[cfg(test)]
#[path = "compositor_window_id_tests.rs"]
mod tests;
