use std::cell::{Cell, RefCell};
use std::time::Duration;

use agent_desktop_core::{ErrorCode, ProcessId, WindowState};

use super::*;
use crate::system::process_facts::ProcessFacts;
use crate::system::x11::desktop_placement::DesktopPlacement;
use crate::system::x11::stacked_window::StackedWindow;
use crate::system::x11::stacking_snapshot::StackingSnapshot;
use crate::system::x11::window_flags::WindowFlags;
use crate::system::x11::window_kind::WindowKind;
use crate::system::x11::{FrameExtents, X11Client};

const CLIENT: u32 = 0x0040_0003;
const OTHER: u32 = 0x0060_0001;
const INSTANCE: &str = "abcd1234:99";

#[derive(Clone, Copy)]
enum Grant {
    Never,
    Now,
    OnlyOnX11,
    AfterReads(u32),
    FrameAfterReads(u32),
}

struct World {
    session: Session,
    lookup: Lookup,
    minimized: Option<bool>,
    state: X11State,
    flags: WindowFlags,
    global: bool,
    grant: Grant,
    send_fails: bool,
    active: Cell<Option<u32>>,
    frame_active: Cell<Option<bool>>,
    sent: RefCell<Vec<u32>>,
    active_reads: Cell<u32>,
    overview: Option<bool>,
}

fn facts() -> ProcessFacts {
    ProcessFacts {
        comm: "app".to_owned(),
        exe_basename: None,
        instance: INSTANCE.to_owned(),
        web_family: None,
    }
}

fn x11_client(id: u32, title: &str) -> X11Client {
    X11Client {
        id,
        hidden: false,
        pid: Some(100),
        title: title.to_owned(),
        extents: None,
        bounds: None,
    }
}

impl World {
    fn new(active: Option<u32>) -> Self {
        Self {
            session: Session::X11,
            lookup: Lookup::Found(facts()),
            minimized: Some(false),
            state: X11State::Ready {
                active,
                clients: vec![x11_client(CLIENT, "Doc")],
            },
            flags: WindowFlags {
                hidden: false,
                above: false,
                mapped: true,
            },
            global: true,
            grant: Grant::Now,
            send_fails: false,
            active: Cell::new(active),
            frame_active: Cell::new(Some(active == Some(CLIENT))),
            sent: RefCell::new(Vec::new()),
            active_reads: Cell::new(0),
            overview: None,
        }
    }

    fn sends(&self) -> usize {
        self.sent.borrow().len()
    }
}

impl FocusWorld for World {
    fn session(&self) -> Session {
        self.session
    }

    fn lookup(&self, _pid: u32) -> Lookup {
        self.lookup.clone()
    }

    fn frame(&self) -> Result<FrameView, AdapterError> {
        Ok(FrameView {
            minimized: self.minimized,
            active: self.frame_active.get(),
        })
    }

    fn clients(&self) -> X11State {
        self.state.clone()
    }

    fn stacking(&self, client: u32) -> StackingRead {
        StackingRead::Ready(StackingSnapshot {
            current_desktop: Some(0),
            stack: vec![StackedWindow {
                id: client,
                flags: self.flags,
                desktop: DesktopPlacement::On(0),
                kind: WindowKind::Normal,
                client: crate::tree::Extents {
                    x: 0,
                    y: 0,
                    width: 100,
                    height: 100,
                },
                extents: FrameExtents {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 0,
                },
            }],
            override_redirect_above: Vec::new(),
            override_redirect_receiving: Vec::new(),
            override_redirect_names: Vec::new(),
        })
    }

    fn global_position(&self) -> bool {
        self.global
    }

    fn send(&self, client: u32) -> Result<(), ActivateError> {
        self.sent.borrow_mut().push(client);
        if self.send_fails {
            return Err(ActivateError::SendFailed);
        }
        match self.grant {
            Grant::Now => {
                self.active.set(Some(client));
                self.frame_active.set(Some(true));
            }
            Grant::OnlyOnX11 | Grant::FrameAfterReads(_) => self.active.set(Some(client)),
            Grant::Never | Grant::AfterReads(_) => {}
        }
        Ok(())
    }

    fn active(&self) -> Option<u32> {
        let reads = self.active_reads.get() + 1;
        self.active_reads.set(reads);
        if let Grant::AfterReads(needed) = self.grant
            && reads >= needed
            && self.sends() > 0
        {
            self.active.set(Some(CLIENT));
            self.frame_active.set(Some(true));
        }
        if let Grant::FrameAfterReads(needed) = self.grant
            && reads >= needed
            && self.sends() > 0
        {
            self.frame_active.set(Some(true));
        }
        self.active.get()
    }

    fn overview_open(&self) -> Option<bool> {
        self.overview
    }
}

fn window() -> WindowInfo {
    WindowInfo {
        id: window_id::encode(":1.7", "/org/a11y/atspi/accessible/5").expect("an id"),
        title: "Doc".to_owned(),
        app: "app".to_owned(),
        pid: ProcessId::new(100),
        process_instance: Some(INSTANCE.to_owned()),
        bounds: None,
        state: WindowState::default(),
    }
}

fn budget() -> Deadline {
    Deadline::after(250).expect("a deadline builds")
}

fn failure(world: &World) -> AdapterError {
    run(world, &window(), budget()).expect_err("the focus must fail")
}

fn assert_refused(error: &AdapterError, cause: &str) {
    assert_eq!(error.code, ErrorCode::ActionFailed);
    let details = error.details.as_ref().expect("details");
    assert_eq!(details["cause"], cause);
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_window_already_active_by_both_measures_sends_nothing() {
    let world = World::new(Some(CLIENT));
    run(&world, &window(), budget()).expect("already active is a success");
    assert_eq!(world.sends(), 0);
}

#[test]
fn a_background_window_is_sent_the_request_and_succeeds_when_both_agree() {
    let world = World::new(Some(OTHER));
    run(&world, &window(), budget()).expect("a granted activation succeeds");
    assert_eq!(*world.sent.borrow(), vec![CLIENT]);
}

#[test]
fn a_grant_that_arrives_after_some_polls_is_still_a_success() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::AfterReads(3);
    run(&world, &window(), budget()).expect("the wait polls rather than checking once");
    assert_eq!(world.sends(), 1);
    assert!(world.active_reads.get() >= 3);
}

#[test]
fn a_hidden_window_is_refused_before_any_send() {
    let mut world = World::new(Some(OTHER));
    world.flags.hidden = true;
    assert_refused(&failure(&world), "window_hidden");
    assert_eq!(world.sends(), 0);
}

#[test]
fn an_unmapped_window_is_refused_before_any_send() {
    let mut world = World::new(Some(OTHER));
    world.flags.mapped = false;
    assert_refused(&failure(&world), "window_not_mapped");
    let mut none = World::new(Some(OTHER));
    none.state = X11State::Ready {
        active: Some(OTHER),
        clients: Vec::new(),
    };
    assert_refused(&failure(&none), "window_not_mapped");
    assert_eq!(world.sends() + none.sends(), 0);
}

#[test]
fn a_minimized_frame_is_refused_before_any_send() {
    let mut world = World::new(Some(OTHER));
    world.minimized = Some(true);
    assert_refused(&failure(&world), "window_minimized");
    assert_eq!(world.sends(), 0);
}

#[test]
fn an_ambiguous_mapping_is_refused_before_any_send() {
    let mut world = World::new(Some(OTHER));
    world.global = false;
    world.state = X11State::Ready {
        active: Some(OTHER),
        clients: vec![x11_client(CLIENT, "Doc"), x11_client(CLIENT + 1, "Doc")],
    };
    assert_refused(&failure(&world), "ambiguous_window");
    assert_eq!(world.sends(), 0);
}

#[test]
fn wayland_no_display_and_no_manager_are_refused_before_any_send() {
    let mut wayland = World::new(Some(OTHER));
    wayland.session = Session::Wayland;
    assert_refused(&failure(&wayland), "wayland_session");
    let mut none = World::new(Some(OTHER));
    none.session = Session::NoDisplay;
    assert_refused(&failure(&none), "no_display");
    let mut bare = World::new(Some(OTHER));
    bare.state = X11State::NoWindowManager;
    assert_refused(&failure(&bare), "no_window_manager");
    let mut dark = World::new(Some(OTHER));
    dark.state = X11State::Unavailable;
    assert_refused(&failure(&dark), "x11_unavailable");
    assert_eq!(
        wayland.sends() + none.sends() + bare.sends() + dark.sends(),
        0
    );
}

#[test]
fn active_on_the_display_but_not_on_the_frame_is_a_failure_never_a_success() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::OnlyOnX11;
    assert_refused(&failure(&world), "frame_not_active");
}

#[test]
fn a_manager_that_ignores_the_request_leaves_the_old_window_active() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::Never;
    assert_refused(&failure(&world), "activation_refused");
    assert_eq!(world.active.get(), Some(OTHER));
}

#[test]
fn a_window_that_never_held_focus_is_not_granted_and_nothing_else_is_sent() {
    let mut world = World::new(None);
    world.grant = Grant::Never;
    let error = failure(&world);
    assert_refused(&error, "activation_not_granted");
    assert!(
        error
            .suggestion
            .as_deref()
            .is_some_and(|text| !text.contains("never held focus"))
    );
    assert_eq!(world.sends(), 1);
}

#[test]
fn the_same_window_after_another_became_active_is_granted() {
    let world = World::new(Some(OTHER));
    run(&world, &window(), budget()).expect("honoured once another window is active");
}

#[test]
fn a_request_that_cannot_be_sent_is_a_not_delivered_failure() {
    let mut world = World::new(Some(OTHER));
    world.send_fails = true;
    assert_refused(&failure(&world), "activation_not_sent");
}

#[test]
fn another_process_instance_ends_the_ref_before_any_send() {
    let mut world = World::new(Some(OTHER));
    let mut changed = facts();
    changed.instance = "ffff0000:1".to_owned();
    world.lookup = Lookup::Found(changed);
    assert_eq!(failure(&world).code, ErrorCode::StaleRef);
    world.lookup = Lookup::Gone;
    assert_eq!(failure(&world).code, ErrorCode::StaleRef);
    assert_eq!(world.sends(), 0);
}

#[test]
fn an_unreadable_process_is_neither_gone_nor_changed_so_focus_goes_on() {
    let mut world = World::new(Some(OTHER));
    world.lookup = Lookup::Unreadable;
    run(&world, &window(), budget()).expect("an unreadable process does not end the ref");
    assert_eq!(world.sends(), 1);
}

#[test]
fn a_window_id_of_another_format_is_refused_before_any_send() {
    let world = World::new(Some(OTHER));
    let mut foreign = window();
    foreign.id = "0x400003".to_owned();
    let error = run(&world, &foreign, budget()).expect_err("a foreign id is refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(world.sends(), 0);
}

#[test]
fn a_minimized_or_hidden_window_is_pointed_to_the_restore_command() {
    for cause in [Cause::WindowMinimized, Cause::WindowHidden] {
        let suggestion = refusal(cause).suggestion.expect("a suggestion");
        assert!(suggestion.contains("'restore'"), "{suggestion}");
    }
}

#[path = "focus_overview_tests.rs"]
mod overview;

const SHORT: Grace = Grace {
    activation: Duration::from_millis(100),
    frame_report: Duration::from_millis(1_500),
};

#[test]
fn a_frame_that_reports_active_after_the_first_grace_is_waited_for_when_the_manager_already_names_it()
 {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::FrameAfterReads(20);
    let wide = Deadline::after(5_000).expect("a deadline builds");

    run_with(&world, &window(), wide, SHORT).expect("the frame reports in time");

    assert!(world.active_reads.get() >= 20);
}

#[test]
fn a_manager_that_never_names_the_window_is_not_waited_for_past_the_first_grace() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::Never;
    let wide = Deadline::after(5_000).expect("a deadline builds");
    let started = std::time::Instant::now();

    let error = run_with(&world, &window(), wide, SHORT).expect_err("refused");

    assert_refused(&error, "activation_refused");
    assert!(started.elapsed() < Duration::from_millis(1_000));
}

#[test]
fn a_frame_that_never_reports_active_is_refused_when_the_longer_grace_ends() {
    let mut world = World::new(Some(OTHER));
    world.grant = Grant::OnlyOnX11;
    let wide = Deadline::after(5_000).expect("a deadline builds");

    let error = run_with(&world, &window(), wide, SHORT).expect_err("refused");

    assert_refused(&error, "frame_not_active");
}
