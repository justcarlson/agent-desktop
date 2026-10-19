use crate::system::toolkit_id::ToolkitId;

/// Whether the toolkit reports screen positions that are global.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) fn reports_global_position(_toolkit: &ToolkitId) -> bool {
    false
}
