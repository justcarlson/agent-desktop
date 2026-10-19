use x11rb::protocol::xproto;

use super::MapState;

#[test]
fn the_three_wire_values_decode_to_the_three_states() {
    assert_eq!(
        MapState::from(xproto::MapState::VIEWABLE),
        MapState::Viewable
    );
    assert_eq!(
        MapState::from(xproto::MapState::UNVIEWABLE),
        MapState::Unviewable
    );
    assert_eq!(
        MapState::from(xproto::MapState::UNMAPPED),
        MapState::Unmapped
    );
}
