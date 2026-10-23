use super::*;

pub(super) fn confirm_rectangle(
    world: &impl OpWorld,
    client: u32,
    first: WindowFacts,
    deadline: Deadline,
) -> (bool, Option<WindowFacts>) {
    if deadline.remaining() < CONFIRM_PAUSE {
        return (false, Some(first));
    }
    std::thread::sleep(CONFIRM_PAUSE);
    let next = world.facts(client).ready();
    let stable = !deadline.is_expired() && next.is_some_and(|next| next.client == first.client);
    (stable, next)
}

/// Puts the client at `place`, with a second try through a place 40 px away
/// where the manager skips a move to the place it believes the window holds.
/// The last move is always the one back to `place`, so a manager that clamps
/// one axis of it still gives the other axis back.
pub(super) fn move_client(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    place: Extents,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let back = move_to(place.x, place.y);
    let there = matches!(
        world.facts(client),
        ControlRead::Ready(facts)
            if (facts.client.x - place.x).abs() <= POSITION_TOLERANCE
                && (facts.client.y - place.y).abs() <= POSITION_TOLERANCE
                && confirm_rectangle(world, client, facts, deadline).0
    );
    if there || apply(world, win, client, &back, deadline).is_ok() {
        return Ok(());
    }
    let nudged = apply(
        world,
        win,
        client,
        &move_to(place.x + NUDGE, place.y),
        deadline,
    );
    apply(world, win, client, &back, deadline).map_err(|error| nudged.err().unwrap_or(error))
}

/// Moves the client back to `place` after a resize. The resize is judged by
/// its size alone, so a move the manager will not take never fails it.
pub(super) fn try_give_back(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    place: Extents,
    deadline: Deadline,
) {
    let _ = move_client(world, win, client, place, deadline);
}

pub(super) fn give_back_place(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let Some(place) = world.remembered(win, client) else {
        return Ok(());
    };
    move_client(world, win, client, place, deadline)?;
    world.remember(win, client, None);
    Ok(())
}

/// Whether a window of this size can sit on one display. A window wider or
/// taller than every display is placed by the manager as it likes, so the
/// place it was given is no measure of it. Without a display list the size is
/// taken to fit.
fn fits_a_display(window: &Extents, displays: &[Rect]) -> bool {
    displays.is_empty()
        || displays.iter().any(|display| {
            f64::from(window.width) <= display.width && f64::from(window.height) <= display.height
        })
}

fn inside_a_display(window: &Extents, displays: &[Rect]) -> bool {
    let slack = f64::from(POSITION_TOLERANCE);
    let (left, top) = (f64::from(window.x), f64::from(window.y));
    let right = left + f64::from(window.width);
    let bottom = top + f64::from(window.height);
    displays.is_empty()
        || displays.iter().any(|display| {
            left >= display.x - slack
                && top >= display.y - slack
                && right <= display.x + display.width + slack
                && bottom <= display.y + display.height + slack
        })
}

/// Whether a resize that left the window as `held` is followed by a move back
/// to the origin it had in `place`. A manager that resizes about its frame
/// shifts the client, and the origin is given back. The origin stays the
/// manager's where the held size fits no display, and where the manager moved
/// the window onto a display only because the place would have taken it off.
pub(super) fn gives_back(place: &WindowFacts, held: &WindowFacts, displays: &[Rect]) -> bool {
    let moved = (held.client.x - place.client.x).abs() > POSITION_TOLERANCE
        || (held.client.y - place.client.y).abs() > POSITION_TOLERANCE;
    let (first, drawn) = (place.visible(), held.visible());
    let first_place = Extents {
        x: first.x,
        y: first.y,
        ..drawn
    };
    let kept_on_display =
        inside_a_display(&drawn, displays) && !inside_a_display(&first_place, displays);
    moved && fits_a_display(&drawn, displays) && !kept_on_display
}

pub(super) fn is_resize(write: &WindowWrite) -> bool {
    matches!(
        write,
        WindowWrite::MoveResize {
            x: None,
            y: None,
            width: Some(_),
            height: Some(_)
        }
    )
}

pub(super) fn move_to(x: i32, y: i32) -> WindowWrite {
    WindowWrite::MoveResize {
        x: Some(x),
        y: Some(y),
        width: None,
        height: None,
    }
}

pub(super) fn remember_place(world: &impl OpWorld, win: &WindowInfo, client: u32) {
    if let ControlRead::Ready(facts) = world.facts(client)
        && !facts.maximized_vertical
        && !facts.maximized_horizontal
        && !facts.hidden
    {
        world.remember(win, client, Some(facts.client));
    }
}
