use super::*;

#[test]
fn an_accepted_scroll_followed_by_no_bar_is_not_retry_safe() {
    let mut waiter = clock(10_000);
    let mut port = Fake::new(waiter.clone());
    port.toolkit = "Chromium";
    port.scroll_to_answer = true;
    port.bar = BarFound::Missing { container: true };
    let error = drive(&mut port, &mut waiter).expect_err("target never appeared");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(port.scroll_to_calls, 1);
    assert!(port.writes.is_empty());
}

#[test]
fn completed_bar_writes_that_do_not_reveal_the_target_are_not_retry_safe() {
    let mut waiter = clock(60_000);
    let mut port = Fake::new(waiter.clone());
    port.moves_on_write = false;
    let error = drive(&mut port, &mut waiter).expect_err("target stays offscreen");
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert!(!port.writes.is_empty());
}

#[test]
fn unknown_visibility_with_valid_bounds_never_fabricates_delivery() {
    let mut waiter = clock(5_000);
    let mut port = Fake::new(waiter.clone());
    port.view.offscreen = None;
    let error = drive(&mut port, &mut waiter).expect_err("visibility is unknown");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(cause(&error), "target_visibility_unknown");
    assert_eq!(port.scroll_to_calls, 0);
    assert!(port.writes.is_empty());
}

#[test]
fn a_read_error_after_a_bar_write_preserves_uncertain_delivery() {
    let mut waiter = clock(5_000);
    let mut port = Fake::new(waiter.clone());
    port.observe_error = true;
    let error = drive(&mut port, &mut waiter).expect_err("read failed after write");
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    assert_eq!(port.writes, [5526.0]);
}
