use std::collections::VecDeque;

use agent_desktop_core::{DisplayInfo, ErrorCode, ProcessId, Rect, WindowState};

use super::*;
use crate::system::hyprland::capture::capture_world::CaptureWorld;
use crate::system::hyprland::capture::hypr_capture::capture_window;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;
use crate::system::wayland::capture::composition::Composition;
use crate::system::wayland::capture::raw_frame::RawFrame;
use crate::system::wayland::capture::shm_layout::XRGB8888;

const INSTANCE: &str = "boot:100";

struct FakeReads {
    clients: VecDeque<Vec<HyprClient>>,
    toplevels: Vec<ToplevelView>,
    live: VecDeque<Option<String>>,
    frames: usize,
}

impl ToplevelReads for FakeReads {
    fn clients(&mut self) -> Result<Vec<HyprClient>, AdapterError> {
        Ok(next(&mut self.clients))
    }

    fn toplevels(&mut self) -> Result<Vec<ToplevelView>, AdapterError> {
        Ok(self.toplevels.clone())
    }

    fn fresh_toplevels(&mut self) -> Result<Vec<ToplevelView>, AdapterError> {
        Ok(self.toplevels.clone())
    }

    fn live_instance(&self, _pid: u32) -> Option<String> {
        self.live.front().cloned().flatten()
    }

    fn compositor(&self) -> (u32, u64) {
        (50, 60)
    }
}

impl CaptureWorld for FakeReads {
    fn enforced(&mut self) -> Result<bool, AdapterError> {
        Ok(false)
    }

    fn bind(&mut self, win: &WindowInfo) -> Result<BoundToplevel, AdapterError> {
        bind_stable(win, self)
    }

    fn composition(&mut self, _bound: &BoundToplevel) -> Result<Composition, AdapterError> {
        Ok(Composition::Current)
    }

    fn capture_toplevel(&mut self, _: &str, _: Composition) -> Result<RawFrame, AdapterError> {
        self.frames += 1;
        if self.live.len() > 1 {
            self.live.pop_front();
        }
        Ok(RawFrame {
            data: vec![0x40; 400 * 300 * 4],
            format: XRGB8888,
            width: 400,
            height: 300,
            stride: 400 * 4,
            inverted: false,
        })
    }

    fn still_bound(
        &mut self,
        win: &WindowInfo,
        bound: &BoundToplevel,
    ) -> Result<bool, AdapterError> {
        still_bound_to(win, bound, self)
    }

    fn displays(&mut self) -> Result<Vec<DisplayInfo>, AdapterError> {
        Ok(Vec::new())
    }

    fn capture_output(&mut self, _: &str) -> Result<RawFrame, AdapterError> {
        Err(AdapterError::internal("no output capture here"))
    }
}

fn next<T: Clone + Default>(answers: &mut VecDeque<T>) -> T {
    if answers.len() > 1 {
        answers.pop_front().unwrap_or_default()
    } else {
        answers.front().cloned().unwrap_or_default()
    }
}

fn client(pid: u32, class: &str) -> HyprClient {
    HyprClient {
        address: 0x10,
        pid,
        title: "Notes".into(),
        class: class.into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn notes() -> HyprClient {
    client(7, "notes")
}

fn other_app() -> HyprClient {
    client(9, "imv")
}

fn window() -> WindowInfo {
    WindowInfo {
        id: "w-1.2-3".into(),
        title: "Notes".into(),
        app: "notes".into(),
        pid: ProcessId::new(7),
        process_instance: Some(INSTANCE.into()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn reads(clients: Vec<Vec<HyprClient>>, live: Vec<Option<&str>>) -> FakeReads {
    FakeReads {
        clients: clients.into(),
        toplevels: vec![ToplevelView {
            identifier: "t1".into(),
            address: 0x10,
        }],
        live: live
            .into_iter()
            .map(|token| token.map(String::from))
            .collect(),
        frames: 0,
    }
}

#[test]
fn an_unchanged_window_is_captured() {
    let mut world = reads(vec![vec![notes()]], vec![Some(INSTANCE)]);
    let image = capture_window(&mut world, &window()).unwrap();
    assert_eq!((image.width, world.frames), (400, 1));
}

#[test]
fn another_process_reusing_the_address_across_the_toplevel_read_is_refused_before_any_frame() {
    let mut world = reads(vec![vec![notes()], vec![other_app()]], vec![Some(INSTANCE)]);
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(world.frames, 0);
}

#[test]
fn another_process_taking_the_address_during_the_frame_returns_no_pixels() {
    let mut world = reads(
        vec![vec![notes()], vec![notes()], vec![other_app()]],
        vec![Some(INSTANCE)],
    );
    let error = capture_window(&mut world, &window()).unwrap_err();
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(world.frames, 1);
}

#[test]
fn a_process_replaced_during_the_frame_returns_no_pixels() {
    for after in [Some("boot:200"), None] {
        let mut world = reads(vec![vec![notes()]], vec![Some(INSTANCE), after]);
        let error = capture_window(&mut world, &window()).unwrap_err();
        assert_eq!(error.code, ErrorCode::WindowNotFound);
        assert_eq!(world.frames, 1);
    }
}
