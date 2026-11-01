use super::*;

fn fullscreen(address: u64, at: Rect) -> StackClient {
    let mut other = client(address, at, "1");
    other.stacking.fullscreen = true;
    other
}

fn over_fullscreen(mut client: StackClient, allowed: Option<bool>) -> StackClient {
    client.stacking.over_fullscreen = allowed;
    client
}

fn whole_output() -> Rect {
    rect(0.0, 0.0, 1280.0, 800.0)
}

#[test]
fn a_fullscreen_window_listed_before_a_floating_target_still_covers() {
    let mut stack = stack();
    stack.clients[0] = over_fullscreen(floating(TARGET), Some(false));
    stack.clients.insert(0, fullscreen(0xb2, whole_output()));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_floating_target_allowed_over_fullscreen_stays_above_a_later_fullscreen_window() {
    let mut stack = stack();
    stack.clients[0] = over_fullscreen(floating(TARGET), Some(true));
    stack.clients.push(fullscreen(0xb2, whole_output()));
    assert_eq!(check(&stack), Ok(()));
}

#[test]
fn a_floating_window_allowed_over_fullscreen_covers_a_later_fullscreen_target() {
    let mut stack = stack();
    stack.clients[0] = fullscreen(TARGET, whole_output());
    stack
        .clients
        .insert(0, over_fullscreen(floating(0xb2), Some(true)));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_floating_window_not_allowed_over_fullscreen_stays_below_a_fullscreen_target() {
    let mut stack = stack();
    stack.clients[0] = fullscreen(TARGET, whole_output());
    stack
        .clients
        .push(over_fullscreen(floating(0xb2), Some(false)));
    assert_eq!(check(&stack), Ok(()));
}

#[test]
fn an_unreported_over_fullscreen_state_where_floating_meets_fullscreen_refuses() {
    let missing = Err(ReceiverRefusal::EvidenceMissing {
        check: "over_fullscreen",
    });
    let mut floating_under = stack();
    floating_under.clients[0] = over_fullscreen(floating(TARGET), None);
    floating_under
        .clients
        .push(fullscreen(0xb2, whole_output()));
    assert_eq!(check(&floating_under), missing);
    let mut fullscreen_under = stack();
    fullscreen_under.clients[0] = fullscreen(TARGET, whole_output());
    fullscreen_under
        .clients
        .push(over_fullscreen(floating(0xb2), None));
    assert_eq!(check(&fullscreen_under), missing);
}

#[test]
fn an_unreported_over_fullscreen_state_still_covers_a_tiled_target() {
    let mut stack = stack();
    stack
        .clients
        .insert(0, over_fullscreen(floating(0xb2), None));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_floating_window_allowed_over_fullscreen_covers_a_later_floating_target() {
    let mut stack = stack();
    stack.clients[0] = over_fullscreen(floating(TARGET), Some(false));
    stack
        .clients
        .insert(0, over_fullscreen(floating(0xb2), Some(true)));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn a_fullscreen_window_on_the_target_workspace_takes_the_pointer_beside_its_rect() {
    let mut stack = stack();
    stack
        .clients
        .push(fullscreen(0xb2, rect(600.0, 400.0, 100.0, 100.0)));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::WindowCover { address: 0xb2 })
    );
}

#[test]
fn an_unreported_over_fullscreen_state_between_floating_windows_refuses() {
    let mut stack = stack();
    stack.clients[0] = floating(TARGET);
    stack
        .clients
        .insert(0, over_fullscreen(floating(0xb2), None));
    assert_eq!(
        check(&stack),
        Err(ReceiverRefusal::EvidenceMissing {
            check: "over_fullscreen"
        })
    );
}
