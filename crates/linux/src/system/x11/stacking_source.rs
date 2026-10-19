use super::stacking_read::StackingRead;

/// Where stacking evidence comes from. The real source talks to the display
/// server; tests supply a fixed answer. `target_client` selects which
/// override-redirect windows count as above.
#[expect(dead_code, reason = "stub the owning lane replaces")]
pub(crate) trait StackingSource: Send + Sync {
    fn read(&self, target_client: u32) -> StackingRead;
}
