use super::*;
use crate::tree::resolve_fake::Fake;

#[test]
fn a_childless_native_frame_is_conclusively_empty() {
    let fake = Fake::new();
    let mut entry = fake.entry_of("/frame", true);
    entry.identity.name = Some("window".to_owned());
    assert!(!evaluate(&fake.walk(), &entry, false).unread_region);
}
