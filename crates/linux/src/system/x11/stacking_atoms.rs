use x11rb::protocol::xproto::Atom;
use x11rb::rust_connection::RustConnection;

use crate::system::x11::stacking_decode::{StateAtoms, TypeAtoms};

const NAMES: [&[u8]; 18] = [
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
    b"_NET_FRAME_EXTENTS",
    b"_GTK_FRAME_EXTENTS",
];

/// The atoms read off the root window.
pub(super) struct RootAtoms {
    pub(super) wm_check: Atom,
    pub(super) stacking: Atom,
    pub(super) current_desktop: Atom,
}

/// The atoms read off each client.
pub(super) struct WindowAtoms {
    pub(super) state: Atom,
    pub(super) desktop: Atom,
    pub(super) kind: Atom,
    pub(super) frame_extents: Atom,
    pub(super) gtk_extents: Atom,
    pub(super) states: StateAtoms,
    pub(super) types: TypeAtoms,
}

pub(super) struct Names {
    pub(super) root: RootAtoms,
    pub(super) window: WindowAtoms,
}

pub(super) fn names(connection: &RustConnection) -> Option<Names> {
    let ids: [Atom; 18] = super::control_display::atoms(connection, &NAMES)?
        .try_into()
        .ok()?;
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
        frame_extents,
        gtk_extents,
    ] = ids;
    Some(Names {
        root: RootAtoms {
            wm_check,
            stacking,
            current_desktop,
        },
        window: WindowAtoms {
            state,
            desktop,
            kind,
            frame_extents,
            gtk_extents,
            states: StateAtoms { hidden, above },
            types: TypeAtoms {
                normal,
                dialog,
                utility,
                menu: [menu, dropdown, popup],
                dock,
                desktop: desktop_type,
            },
        },
    })
}
