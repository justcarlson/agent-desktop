use agent_desktop_core::Point;

use super::Placement;

#[test]
fn the_hotspot_lands_on_the_rounded_point() {
    let placement = Placement::for_point(&Point { x: 100.4, y: 200.6 }, (1.0, 1.0));
    assert_eq!(placement.margin, (99, 200));
    assert_eq!(placement.output, None);
}
