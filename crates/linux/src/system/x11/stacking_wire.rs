use x11rb::connection::Connection;
use x11rb::cookie::Cookie;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ConnectionExt, GetGeometryReply, GetPropertyReply, GetWindowAttributesReply,
    MapState, TranslateCoordinatesReply, Window,
};
use x11rb::rust_connection::RustConnection;

use crate::system::x11::desktop_placement::DesktopPlacement;
use crate::system::x11::snapshot::decode_frame_extents;
use crate::system::x11::stacked_window::StackedWindow;
use crate::system::x11::stacking_decode::{
    StateAtoms, TypeAtoms, decode_current_desktop, decode_desktop, decode_kind, decode_stack,
    decode_state,
};
use crate::system::x11::stacking_read::StackingRead;
use crate::system::x11::stacking_snapshot::StackingSnapshot;
use crate::system::x11::window_flags::WindowFlags;
use crate::system::x11::window_kind::WindowKind;
use crate::tree::Extents;

const MAX_STACK: u32 = 4_096;
const MAX_ENTRIES: usize = 256;
const MAX_OVERLAYS: usize = 64;
const TYPE_LONGS: u32 = 8;
const STATE_LONGS: u32 = 64;

const NAMES: [&[u8]; 16] = [
    b"_NET_SUPPORTING_WM_CHECK",
    b"_NET_CLIENT_LIST_STACKING",
    b"_NET_CURRENT_DESKTOP",
    b"_NET_WM_STATE",
    b"_NET_WM_STATE_HIDDEN",
    b"_NET_WM_STATE_ABOVE",
    b"_NET_WM_DESKTOP",
    b"_NET_WM_WINDOW_TYPE",
    b"_NET_WM_WINDOW_TYPE_NORMAL",
    b"_NET_WM_WINDOW_TYPE_DIALOG",
    b"_NET_WM_WINDOW_TYPE_UTILITY",
    b"_NET_WM_WINDOW_TYPE_MENU",
    b"_NET_WM_WINDOW_TYPE_DROPDOWN_MENU",
    b"_NET_WM_WINDOW_TYPE_POPUP_MENU",
    b"_NET_WM_WINDOW_TYPE_DOCK",
    b"_NET_WM_WINDOW_TYPE_DESKTOP",
];

struct Names {
    wm_check: Atom,
    stacking: Atom,
    current_desktop: Atom,
    state: Atom,
    desktop: Atom,
    kind: Atom,
    frame_extents: Atom,
    states: StateAtoms,
    types: TypeAtoms,
}

fn names(connection: &RustConnection) -> Option<Names> {
    let cookies: Vec<_> = NAMES
        .iter()
        .map(|name| connection.intern_atom(false, name).ok())
        .collect::<Option<_>>()?;
    let ids: Vec<Atom> = cookies
        .into_iter()
        .map(|cookie| cookie.reply().ok().map(|reply| reply.atom))
        .collect::<Option<_>>()?;
    let frame_extents = connection
        .intern_atom(false, b"_NET_FRAME_EXTENTS")
        .ok()?
        .reply()
        .ok()?
        .atom;
    let [
        wm_check,
        stacking,
        current_desktop,
        state,
        hidden,
        above,
        desktop,
        kind,
        normal,
        dialog,
        utility,
        menu,
        dropdown,
        popup,
        dock,
        desktop_type,
    ] = ids.as_slice()
    else {
        return None;
    };
    Some(Names {
        wm_check: *wm_check,
        stacking: *stacking,
        current_desktop: *current_desktop,
        state: *state,
        desktop: *desktop,
        kind: *kind,
        frame_extents,
        states: StateAtoms {
            hidden: *hidden,
            above: *above,
        },
        types: TypeAtoms {
            normal: *normal,
            dialog: *dialog,
            utility: *utility,
            menu: [*menu, *dropdown, *popup],
            dock: *dock,
            desktop: *desktop_type,
        },
    })
}

type Property<'c> = Cookie<'c, RustConnection, GetPropertyReply>;

fn property(
    connection: &RustConnection,
    window: Window,
    name: Atom,
    kind: impl Into<Atom>,
    longs: u32,
) -> Option<Property<'_>> {
    connection
        .get_property(false, window, name, kind, 0, longs)
        .ok()
}

fn bytes(cookie: Option<Property<'_>>) -> Option<(u8, Vec<u8>)> {
    let reply = cookie?.reply().ok()?;
    Some((reply.format, reply.value))
}

struct Pending<'c> {
    id: Window,
    state: Option<Property<'c>>,
    desktop: Option<Property<'c>>,
    kind: Option<Property<'c>>,
    extents: Option<Property<'c>>,
    rest: Geometry<'c>,
}

struct Geometry<'c> {
    attributes: Option<Cookie<'c, RustConnection, GetWindowAttributesReply>>,
    origin: Option<Cookie<'c, RustConnection, TranslateCoordinatesReply>>,
    size: Option<Cookie<'c, RustConnection, GetGeometryReply>>,
}

fn geometry(connection: &RustConnection, root: Window, id: Window) -> Geometry<'_> {
    Geometry {
        attributes: connection.get_window_attributes(id).ok(),
        origin: connection.translate_coordinates(id, root, 0, 0).ok(),
        size: connection.get_geometry(id).ok(),
    }
}

fn request<'c>(
    connection: &'c RustConnection,
    names: &Names,
    root: Window,
    id: Window,
) -> Pending<'c> {
    Pending {
        id,
        state: property(connection, id, names.state, AtomEnum::ATOM, STATE_LONGS),
        desktop: property(connection, id, names.desktop, AtomEnum::CARDINAL, 1),
        kind: property(connection, id, names.kind, AtomEnum::ATOM, TYPE_LONGS),
        extents: property(connection, id, names.frame_extents, AtomEnum::CARDINAL, 4),
        rest: geometry(connection, root, id),
    }
}

fn client_rectangle(rest: Geometry<'_>) -> Option<(bool, Extents)> {
    let attributes = rest.attributes?.reply().ok()?;
    let origin = rest.origin?.reply().ok()?;
    let size = rest.size?.reply().ok()?;
    Some((
        attributes.map_state == MapState::VIEWABLE,
        Extents {
            x: i32::from(origin.dst_x),
            y: i32::from(origin.dst_y),
            width: i32::from(size.width),
            height: i32::from(size.height),
        },
    ))
}

fn collect(pending: Pending<'_>, names: &Names) -> Option<StackedWindow> {
    let (mapped, client) = client_rectangle(pending.rest)?;
    let (hidden, above) = bytes(pending.state).map_or((false, false), |(format, value)| {
        decode_state(format, &value, names.states)
    });
    let desktop = bytes(pending.desktop).map_or(DesktopPlacement::Unknown, |(format, value)| {
        decode_desktop(format, &value)
    });
    let kind = bytes(pending.kind).map_or(WindowKind::Normal, |(format, value)| {
        decode_kind(format, &value, names.types)
    });
    let extents = bytes(pending.extents)
        .and_then(|(format, value)| decode_frame_extents(format, &value))
        .unwrap_or(crate::system::x11::FrameExtents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        });
    Some(StackedWindow {
        id: pending.id,
        flags: WindowFlags {
            hidden,
            above,
            mapped,
        },
        desktop,
        kind,
        client,
        extents,
    })
}

fn frame_of(connection: &RustConnection, root: Window, client: Window) -> Option<Window> {
    let parent = connection.query_tree(client).ok()?.reply().ok()?.parent;
    Some(if parent == root || parent == 0 {
        client
    } else {
        parent
    })
}

fn overlays(connection: &RustConnection, root: Window, frame: Window) -> Option<Vec<Extents>> {
    let siblings = connection.query_tree(root).ok()?.reply().ok()?.children;
    let position = siblings.iter().position(|window| *window == frame)?;
    let probes: Vec<_> = siblings[position + 1..]
        .iter()
        .take(MAX_ENTRIES)
        .map(|window| {
            (
                connection.get_window_attributes(*window).ok(),
                connection.get_geometry(*window).ok(),
            )
        })
        .collect();
    let mut found = Vec::new();
    for (attributes, size) in probes {
        let Some(attributes) = attributes.and_then(|cookie| cookie.reply().ok()) else {
            continue;
        };
        if !attributes.override_redirect || attributes.map_state != MapState::VIEWABLE {
            continue;
        }
        let Some(size) = size.and_then(|cookie| cookie.reply().ok()) else {
            continue;
        };
        let border = i32::from(size.border_width) * 2;
        found.push(Extents {
            x: i32::from(size.x),
            y: i32::from(size.y),
            width: i32::from(size.width) + border,
            height: i32::from(size.height) + border,
        });
        if found.len() == MAX_OVERLAYS {
            break;
        }
    }
    Some(found)
}

/// The stacking evidence for one client on the display `display`: the
/// target and every managed client above it, bottom to top, and the mapped
/// override-redirect windows above the target's frame.
pub(super) fn read_stacking(display: &str, target: u32) -> StackingRead {
    let Ok((connection, screen)) = RustConnection::connect(Some(display)) else {
        return StackingRead::Unavailable;
    };
    let Some(root) = connection
        .setup()
        .roots
        .get(screen)
        .map(|screen| screen.root)
    else {
        return StackingRead::Unavailable;
    };
    let Some(names) = names(&connection) else {
        return StackingRead::Unavailable;
    };
    let manager = property(&connection, root, names.wm_check, AtomEnum::WINDOW, 1);
    let stack = property(
        &connection,
        root,
        names.stacking,
        AtomEnum::WINDOW,
        MAX_STACK,
    );
    let desktop = property(
        &connection,
        root,
        names.current_desktop,
        AtomEnum::CARDINAL,
        1,
    );
    let has_manager = bytes(manager).is_some_and(|(_, value)| !value.is_empty());
    if !has_manager {
        return StackingRead::NoManager;
    }
    let Some(listed) = bytes(stack).and_then(|(format, value)| decode_stack(format, &value)) else {
        return StackingRead::Unavailable;
    };
    let Some(position) = listed.iter().position(|window| *window == target) else {
        return StackingRead::Unavailable;
    };
    let pending: Vec<_> = listed[position..]
        .iter()
        .take(MAX_ENTRIES)
        .map(|window| request(&connection, &names, root, *window))
        .collect();
    let current_desktop =
        bytes(desktop).and_then(|(format, value)| decode_current_desktop(format, &value));
    let stack: Vec<StackedWindow> = pending
        .into_iter()
        .filter_map(|entry| collect(entry, &names))
        .collect();
    if stack.first().is_none_or(|window| window.id != target) {
        return StackingRead::Unavailable;
    }
    let Some(above) =
        frame_of(&connection, root, target).and_then(|frame| overlays(&connection, root, frame))
    else {
        return StackingRead::Unavailable;
    };
    StackingRead::Ready(StackingSnapshot {
        current_desktop,
        stack,
        override_redirect_above: above,
    })
}
