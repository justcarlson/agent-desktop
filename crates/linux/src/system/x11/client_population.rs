use std::collections::HashSet;

use agent_desktop_core::Deadline;
use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, MapState, WindowClass};

use super::control::ControlRead;
use super::control_display::{atoms, longs, open, property, route};

/// Prove that the native window population has no extra visible surfaces,
/// including override-redirect popups and embedded top-level windows.
/// An empty population proves that no visible native window remains.
pub(crate) fn complete(pid: u32, clients: Vec<u32>, deadline: Deadline) -> bool {
    route(deadline, move |display| {
        let read = || {
            let (connection, root) = open(&display)?;
            let managed: HashSet<_> = clients.into_iter().collect();
            for client in &managed {
                if super::resource_owner::pid(&connection, *client)? != pid {
                    return Some(false);
                }
            }
            let names = atoms(&connection, &[b"_NET_WM_WINDOW_TYPE"])?;
            let mut pending = vec![(root, false)];
            let mut seen = HashSet::new();
            while let Some((window, ancestor)) = pending.pop() {
                if !seen.insert(window) || seen.len() > 4_096 {
                    return None;
                }
                let covered = ancestor || managed.contains(&window);
                if window != root {
                    let attributes = connection
                        .get_window_attributes(window)
                        .ok()?
                        .reply()
                        .ok()?;
                    if attributes.class == WindowClass::INPUT_OUTPUT
                        && attributes.map_state == MapState::VIEWABLE
                        && super::resource_owner::pid(&connection, window)? == pid
                    {
                        if attributes.override_redirect || !covered {
                            return Some(false);
                        }
                        if !managed.contains(&window) {
                            let kind = property(&connection, window, names[0], AtomEnum::ATOM, 8)?;
                            if !longs(kind.format, &kind.value, kind.bytes_after)?.is_empty() {
                                return Some(false);
                            }
                        }
                    }
                }
                let children = connection.query_tree(window).ok()?.reply().ok()?.children;
                pending.extend(children.into_iter().map(|child| (child, covered)));
            }
            Some(managed.is_subset(&seen))
        };
        read().map_or(ControlRead::Unavailable, ControlRead::Ready)
    })
    .ready()
        == Some(true)
}
