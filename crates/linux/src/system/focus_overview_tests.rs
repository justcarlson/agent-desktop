use super::*;

#[test]
fn an_open_overview_is_the_cause_when_nothing_was_granted() {
    let mut world = World::new(None);
    world.grant = Grant::Never;
    world.overview = Some(true);
    let error = failure(&world);
    assert_refused(&error, "overview_open");
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| text.contains("Escape"))
    );
}

#[test]
fn a_closed_or_unreadable_overview_leaves_the_cause_unclaimed() {
    for overview in [Some(false), None] {
        let mut world = World::new(None);
        world.grant = Grant::Never;
        world.overview = overview;
        assert_refused(&failure(&world), "activation_not_granted");
    }
}

#[test]
fn an_open_overview_is_also_the_cause_when_another_window_stayed_active() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::Never;
    world.overview = Some(true);
    assert_refused(&failure(&world), "overview_open");
}

#[test]
fn an_open_overview_does_not_rename_a_window_active_on_the_display_only() {
    let mut world = World::new(None);
    world.grant = Grant::OnlyOnX11;
    world.overview = Some(true);
    assert_refused(&failure(&world), "frame_not_active");
}
