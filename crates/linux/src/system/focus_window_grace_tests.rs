use std::time::Duration;

use agent_desktop_core::Deadline;

use super::{Grant, OTHER, World, assert_refused, window};
use crate::system::focus_window::{Grace, run};

const SHORT: Grace = Grace {
    activation: Duration::from_millis(100),
    frame_report: Duration::from_millis(1_500),
};

#[test]
fn a_frame_that_reports_active_after_the_first_grace_is_waited_for_when_the_manager_already_names_it()
 {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::FrameAfterReads(20);
    world.grace = SHORT;
    let wide = Deadline::after(5_000).expect("a deadline builds");

    run(&world, &window(), wide).expect("the frame reports in time");

    assert!(world.active_reads.get() >= 20);
}

#[test]
fn a_manager_that_never_names_the_window_is_not_waited_for_past_the_first_grace() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::Never;
    world.grace = SHORT;
    let wide = Deadline::after(5_000).expect("a deadline builds");
    let started = std::time::Instant::now();

    let error = run(&world, &window(), wide).expect_err("refused");

    assert_refused(&error, "activation_refused");
    assert!(started.elapsed() < Duration::from_millis(1_000));
}
