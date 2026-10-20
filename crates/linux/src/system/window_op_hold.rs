use super::*;

/// Puts the client at `place`, with a second try through a place 40 px away
/// where the manager skips a move to the place it believes the window holds.
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
    );
    if !there && apply(world, win, client, &back, deadline).is_err() {
        apply(
            world,
            win,
            client,
            &move_to(place.x + NUDGE, place.y),
            deadline,
        )?;
        apply(world, win, client, &back, deadline)?;
    }
    Ok(())
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

/// A resize must not move the window. Some managers resize about the frame
/// and leave the client shifted by the frame's extents, so the place the
/// client had is given back when the resize moved it.
pub(super) fn hold_place(
    world: &impl OpWorld,
    win: &WindowInfo,
    client: u32,
    before: Option<Extents>,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let Some(place) = before else {
        return Ok(());
    };
    let displays = world.displays();
    let moved = matches!(
        world.facts(client),
        ControlRead::Ready(facts)
            if fits_a_display(&facts.client, &displays)
                && ((facts.client.x - place.x).abs() > POSITION_TOLERANCE
                    || (facts.client.y - place.y).abs() > POSITION_TOLERANCE)
    );
    if moved {
        move_client(world, win, client, place, deadline)?;
    }
    Ok(())
}

/// Whether a window of this size can sit on one display. A window wider or
/// taller than every display is placed by the manager as it likes, so the
/// place it was given is no measure of it. Without a display list the size is
/// taken to fit.
fn fits_a_display(client: &Extents, displays: &[Rect]) -> bool {
    displays.is_empty()
        || displays.iter().any(|display| {
            f64::from(client.width) <= display.width && f64::from(client.height) <= display.height
        })
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
