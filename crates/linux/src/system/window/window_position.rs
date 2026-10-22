use agent_desktop_core::Rect;

use super::listed_twins::listed_client;
use crate::system::registry::listed_window::ListedWindow;
use crate::system::x11::desktops::Desktops;
use crate::system::x11::map::ClientMatch;
use crate::system::x11::{X11Client, X11State};
use crate::tree::Extents;

/// Where the X11 client `id` sits, or `None` when it is not listed or its
/// bounds are unknown.
pub(crate) fn origin_of_client(clients: &[X11Client], id: u32) -> Option<(i32, i32)> {
    client_origin(clients.iter().find(|client| client.id == id)?)
}

fn client_origin(client: &X11Client) -> Option<(i32, i32)> {
    let bounds = client.bounds?;
    Some((
        bounds.x.checked_add(client.shadow.0)?,
        bounds.y.checked_add(client.shadow.1)?,
    ))
}

fn visible_size(client: &X11Client) -> Option<Extents> {
    let (left, top, right, bottom) = client.shadow;
    if client.shadow == (0, 0, 0, 0) {
        return None;
    }
    let bounds = client.bounds?;
    Some(Extents {
        x: 0,
        y: 0,
        width: bounds.width.checked_sub(left)?.checked_sub(right)?,
        height: bounds.height.checked_sub(top)?.checked_sub(bottom)?,
    })
}

fn at(origin: (i32, i32), size: Extents) -> Rect {
    Rect::from(Extents {
        x: origin.0,
        y: origin.1,
        width: size.width,
        height: size.height,
    })
}

/// Places and annotates windows using one X11 client match per window.
/// Missing display evidence leaves global bounds and toolkit state intact;
/// a non-global toolkit has no trustworthy position without one unique client.
/// Windows that share a title are told apart as the window manager's active
/// client and the toolkit's active frame allow, and otherwise stay tied.
pub(crate) fn place_windows(
    listed: &mut [ListedWindow],
    state: &X11State,
    desktops: Option<&Desktops>,
) {
    let clients: &[X11Client] = match state {
        X11State::Ready { clients, .. } => clients,
        _ => &[],
    };
    let matched: Vec<Option<&X11Client>> = (0..listed.len())
        .map(|index| match listed_client(listed, index, state) {
            ClientMatch::One(id) => clients.iter().find(|client| client.id == id),
            ClientMatch::Several(_) | ClientMatch::NoClient => None,
        })
        .collect();
    for (window, client) in listed.iter_mut().zip(matched) {
        if !window.frame.global_position {
            window.info.bounds = window.frame.extents.zip(client).and_then(|(size, client)| {
                client_origin(client).map(|origin| at(origin, visible_size(client).unwrap_or(size)))
            });
        } else if let (Some(client), Some(bounds)) = (client, window.info.bounds.as_mut()) {
            let (left, top, right, bottom) = client.shadow;
            bounds.x += f64::from(left);
            bounds.y += f64::from(top);
            bounds.width -= f64::from(left + right);
            bounds.height -= f64::from(top + bottom);
        }
        if client.is_some_and(|client| client.hidden) {
            window.info.state.minimized = Some(true);
            window.info.state.visible = Some(false);
        }
        if let (Some(client), Some(desktops)) = (client, desktops)
            && desktops.hides(client.id)
        {
            window.info.state.visible = Some(false);
        }
    }
}

#[cfg(test)]
#[path = "window_position_tests.rs"]
mod tests;
