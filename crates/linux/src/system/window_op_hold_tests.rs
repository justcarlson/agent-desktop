use super::*;

fn shifted_by_the_frame(written: &[WindowWrite]) -> WindowFacts {
    let moved_back = written.iter().any(|write| {
        matches!(
            write,
            WindowWrite::MoveResize {
                x: Some(300),
                y: Some(200),
                ..
            }
        )
    });
    if moved_back {
        state(300, 200, 520, 380)
    } else if written.is_empty() {
        state(300, 200, 400, 300)
    } else {
        state(301, 222, 520, 380)
    }
}

#[test]
fn a_resize_that_shifts_the_client_gives_the_place_back() {
    let mut world = World::showing(state(300, 200, 400, 300));
    world.reaction = Some(shifted_by_the_frame);
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    run(&world, &window(), &resize(520.0, 380.0), deadline).expect("a resize that holds place");
    let writes = world.writes.borrow();
    assert_eq!(writes.len(), 2);
    assert_eq!(
        writes[1].1,
        WindowWrite::MoveResize {
            x: Some(300),
            y: Some(200),
            width: None,
            height: None
        }
    );
}

#[test]
fn a_resize_that_keeps_the_client_where_it_was_writes_nothing_more() {
    let world = World::showing(state(300, 200, 520, 380));
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    run(&world, &window(), &resize(520.0, 380.0), deadline).expect("a resize");
    assert_eq!(world.writes(), 1);
}

#[test]
fn a_size_that_fits_only_for_a_moment_is_not_a_success() {
    let world = World::showing(state(0, 0, 400, 300))
        .then_showing(state(0, 0, 5000, 5000))
        .then_showing(state(0, 0, 1280, 800));
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    let error = run(&world, &window(), &resize(5000.0, 5000.0), deadline)
        .expect_err("the toolkit clamped the size");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.expect("details");
    assert_eq!(details["observed"]["width"], 1280);
    assert_eq!(details["observed"]["height"], 800);
}

#[test]
fn a_clamped_resize_that_also_shifted_the_client_still_gives_the_place_back() {
    fn clamped_and_shifted(written: &[WindowWrite]) -> WindowFacts {
        let back = written.iter().any(|write| {
            matches!(
                write,
                WindowWrite::MoveResize {
                    x: Some(300),
                    y: Some(200),
                    ..
                }
            )
        });
        match (written.is_empty(), back) {
            (true, _) => state(300, 200, 400, 300),
            (false, true) => state(300, 200, 1280, 800),
            (false, false) => state(301, 222, 1280, 800),
        }
    }
    let mut world = World::showing(state(300, 200, 400, 300));
    world.reaction = Some(clamped_and_shifted);
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    let error = run(&world, &window(), &resize(5000.0, 5000.0), deadline).expect_err("clamped");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(world.writes(), 2);
}

fn oversize_and_shifted(written: &[WindowWrite]) -> WindowFacts {
    if written.is_empty() {
        state(300, 200, 400, 300)
    } else {
        state(-1200, 0, 5000, 5000)
    }
}

#[test]
fn a_resize_beyond_every_display_is_not_judged_by_the_place_the_manager_gave_it() {
    let mut world = World::showing(state(300, 200, 400, 300));
    world.reaction = Some(oversize_and_shifted);
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    run(&world, &window(), &resize(5000.0, 5000.0), deadline).expect("the size was taken");
    assert_eq!(world.writes(), 1);
}
