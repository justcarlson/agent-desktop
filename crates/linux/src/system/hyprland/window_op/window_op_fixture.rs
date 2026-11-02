//! The bound window, compositor clients and frames the window-op tests
//! share.

use agent_desktop_core::{Point, ProcessId, Rect, WindowInfo, WindowState};

use super::client_marks::ClientMarks;
use super::client_state::ClientState;
use super::output_origin::OutputOrigin;
use super::window_world_fake::FakeWindowWorld;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::desktop_state::DesktopState;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::clients::monitor_workspaces::MonitorWorkspaces;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

pub(crate) const TARGET: u64 = 0x10;
pub(crate) const HOLDER: u64 = 0x30;
pub(crate) const LUA_REPLY: &str = "ok";
pub(crate) const LEGACY_REPLY: &str = "eval is only supported with the lua config manager";

pub(crate) fn client(address: u64, pid: u32) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: "foot".into(),
        class: "foot".into(),
        geometry: Rect {
            x: 40.0,
            y: 60.0,
            width: 630.0,
            height: 755.0,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

pub(crate) fn state(address: u64, at: (i64, i64), size: (i64, i64)) -> ClientState {
    ClientState {
        address,
        at,
        size,
        floating: true,
        fullscreen: 0,
        workspace: "1".into(),
        marks: ClientMarks {
            monitor: Some(0),
            tags: Vec::new(),
        },
    }
}

pub(crate) fn desktop(active: Option<u64>) -> DesktopState {
    DesktopState {
        active,
        monitors: vec![MonitorWorkspaces {
            monitor: "Virtual-1".into(),
            id: Some(0),
            active: "1".into(),
            special: None,
        }],
        cursor: Some(Point { x: 322.0, y: 413.0 }),
    }
}

pub(crate) fn holder() -> ClientState {
    state(HOLDER, (650, 10), (620, 780))
}

pub(crate) fn frame(target: ClientState, active: Option<u64>) -> (Vec<ClientState>, DesktopState) {
    (vec![target, holder()], desktop(active))
}

pub(crate) fn world(
    eval_reply: &str,
    frames: Vec<(Vec<ClientState>, DesktopState)>,
) -> FakeWindowWorld {
    FakeWindowWorld {
        inventory: (
            vec![client(TARGET, 7), client(HOLDER, 9)],
            vec![
                ToplevelView {
                    identifier: "a".into(),
                    address: TARGET,
                },
                ToplevelView {
                    identifier: "b".into(),
                    address: HOLDER,
                },
            ],
        ),
        origins: Some(vec![OutputOrigin { id: 0, at: (0, 0) }]),
        frames,
        eval_reply: eval_reply.into(),
        reply: "ok".into(),
        budget: 3,
        log: Vec::new(),
    }
}

pub(crate) fn window() -> WindowInfo {
    WindowInfo {
        id: "w-hypr-50-60-a".into(),
        title: "foot".into(),
        app: "foot".into(),
        pid: ProcessId::new(7),
        process_instance: Some("boot:100".into()),
        bounds: Some(Rect {
            x: 40.0,
            y: 60.0,
            width: 630.0,
            height: 755.0,
        }),
        state: WindowState::default(),
    }
}

pub(crate) fn floating_at(at: (i64, i64)) -> ClientState {
    state(TARGET, at, (630, 755))
}

pub(crate) const MAXIMIZE: &str = "dispatch hl.dsp.window.fullscreen({mode=\"maximized\",action=\"set\",window=\"address:0x10\"})";
pub(crate) const UNSET: &str = "dispatch hl.dsp.window.fullscreen_state({internal=0,client=0,action=\"set\",window=\"address:0x10\"})";
pub(crate) const RECORD: &str = "agent-desktop-restore:40,60,630,755";
pub(crate) const TAG: &str = "dispatch hl.dsp.window.tag({tag=\"+agent-desktop-restore:40,60,630,755\",window=\"address:0x10\"})";
pub(crate) const UNTAG: &str = "dispatch hl.dsp.window.tag({tag=\"-agent-desktop-restore:40,60,630,755\",window=\"address:0x10\"})";

pub(crate) fn tagged(state: ClientState) -> ClientState {
    let mut state = state;
    state.marks.tags = vec![RECORD.into()];
    state
}
