use super::*;

#[test]
fn gtk4_inside_a_client_side_shadow_is_calibrated_from_the_origin_after_the_shadow() {
    let mut shadowed = windowing((302, 260), vec![]);
    shadowed.snapshot.stack[0].shadow = (61, 55);
    let mut world = World::new(gtk4_facts(), Some(shadowed));
    reaching(&mut world);

    assert_eq!(test(&mut world, (800, 620)), HitTestResult::ReachesTarget);
    assert_eq!(world.nodes.points.first(), Some(&(437, 305)));
    let mut outside = World::new(gtk4_facts(), Some(windowing((302, 260), vec![])));
    reaching(&mut outside);
    assert_eq!(test(&mut outside, (800, 620)), HitTestResult::Unknown);
}

#[test]
fn a_hidden_or_unmapped_window_is_unknown() {
    let mut stacking = windowing((100, 137), vec![]);
    stacking.snapshot.stack[0].flags.hidden = true;
    let mut world = World::new(gtk3_facts(), Some(stacking));
    reaching(&mut world);
    assert_eq!(test(&mut world, (310, 154)), HitTestResult::Unknown);
}
