use super::{FrameFacts, X11Client, X11Evidence, X11State};
use crate::tree::Extents;

/// Which X11 clients a listed window could be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClientMatch {
    One(u32),
    Several(Vec<u32>),
    NoClient,
}

fn frame_around(client: &X11Client) -> Option<Extents> {
    let bounds = client.bounds?;
    let top = client.extents.map_or(0, |extents| extents.top);
    let top = i32::try_from(top).unwrap_or(0);
    Some(Extents {
        x: bounds.x,
        y: bounds.y - top,
        width: bounds.width,
        height: bounds.height + top,
    })
}

/// The client of a window, by process id and title first. A tie is broken by
/// the window's extents only when the toolkit reports them in screen
/// coordinates: the accessibility rectangle then equals the client rectangle
/// grown upward by the top frame extent. A tie that extents cannot break
/// stays a tie, and child order is never used.
pub(crate) fn match_client(frame: &FrameFacts, clients: &[X11Client]) -> ClientMatch {
    let named: Vec<&X11Client> = clients
        .iter()
        .filter(|client| client.pid == Some(frame.pid) && client.title == frame.title)
        .collect();
    let [_, _, ..] = named.as_slice() else {
        return named
            .first()
            .map_or(ClientMatch::NoClient, |client| ClientMatch::One(client.id));
    };
    if let (true, Some(extents)) = (frame.global_position, frame.extents) {
        let placed: Vec<&&X11Client> = named
            .iter()
            .filter(|client| frame_around(client) == Some(extents))
            .collect();
        if let [only] = placed.as_slice() {
            return ClientMatch::One(only.id);
        }
    }
    ClientMatch::Several(named.iter().map(|client| client.id).collect())
}

/// What the window manager's answer says about the listed windows.
pub(crate) fn evidence(state: &X11State, frames: &[FrameFacts]) -> X11Evidence {
    let X11State::Ready { active, clients } = state else {
        return X11Evidence::Unavailable;
    };
    let Some(active) = active else {
        return X11Evidence::NoActive;
    };
    let mut exact = Vec::new();
    let mut possible = false;
    for (index, frame) in frames.iter().enumerate() {
        match match_client(frame, clients) {
            ClientMatch::One(id) if id == *active => exact.push(index),
            ClientMatch::Several(ids) if ids.contains(active) => possible = true,
            _ => {}
        }
    }
    match (exact.as_slice(), possible) {
        ([only], false) => X11Evidence::Frame(*only),
        ([], false) => X11Evidence::Unmapped,
        _ => X11Evidence::Ambiguous,
    }
}

#[cfg(test)]
#[path = "map_tests.rs"]
mod tests;
