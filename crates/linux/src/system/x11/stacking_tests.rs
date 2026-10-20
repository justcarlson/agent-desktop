use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use super::*;
use crate::system::display_server::{Cause, Unavailable};
use crate::system::x11::desktop_placement::DesktopPlacement;
use crate::system::x11::stacked_window::StackedWindow;
use crate::system::x11::stacking_snapshot::StackingSnapshot;
use crate::system::x11::window_flags::WindowFlags;
use crate::system::x11::window_kind::WindowKind;
use crate::system::x11::{FrameExtents, X11Client};
use crate::tree::Extents;

fn window(id: u32, hidden: bool, x: i32, y: i32) -> StackedWindow {
    StackedWindow {
        shadow: (0, 0),
        id,
        flags: WindowFlags {
            hidden,
            above: false,
            mapped: true,
        },
        desktop: DesktopPlacement::On(0),
        kind: WindowKind::Normal,
        client: Extents {
            x,
            y,
            width: 300,
            height: 200,
        },
        extents: FrameExtents {
            left: 0,
            right: 0,
            top: 0,
            bottom: 0,
        },
    }
}

struct Fixed {
    clients: X11State,
    stack: StackingRead,
    reads: AtomicU32,
}

impl StackingSource for Fixed {
    fn read_within(&self, _target_client: u32, _deadline: Deadline) -> StackingRead {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.stack.clone()
    }

    fn clients_within(&self, _deadline: Deadline) -> X11State {
        self.clients.clone()
    }
}

fn client(id: u32, pid: u32, title: &str) -> X11Client {
    X11Client {
        shadow: (0, 0),
        id,
        hidden: false,
        pid: Some(pid),
        title: title.to_owned(),
        extents: None,
        bounds: None,
    }
}

fn facts(pid: u32, title: &str) -> FrameFacts {
    FrameFacts {
        pid,
        title: title.to_owned(),
        extents: None,
        global_position: false,
    }
}

fn ready(clients: Vec<X11Client>) -> X11State {
    X11State::Ready {
        active: None,
        clients,
    }
}

fn snapshot(stack: Vec<StackedWindow>) -> StackingRead {
    StackingRead::Ready(StackingSnapshot {
        current_desktop: Some(0),
        stack,
        override_redirect_above: Vec::new(),
        override_redirect_receiving: Vec::new(),
        override_redirect_names: Vec::new(),
    })
}

fn deadline() -> Deadline {
    Deadline::after(2_000).expect("a deadline builds")
}

#[test]
fn a_listed_window_with_one_client_reports_that_clients_state_and_origin() {
    let source = Fixed {
        clients: ready(vec![client(5, 10, "Doc"), client(6, 11, "Other")]),
        stack: snapshot(vec![window(5, true, 120, 80), window(6, false, 0, 0)]),
        reads: AtomicU32::new(0),
    };
    let state = frame_window_state(&source, deadline(), &facts(10, "Doc"));
    assert_eq!(
        state,
        Some(WindowX11State {
            shadow: (0, 0),
            client_id: 5,
            hidden: true,
            client_origin: (120, 80),
        })
    );
}

#[test]
fn a_window_with_no_single_client_has_no_state_and_reads_no_stacking() {
    let two = ready(vec![client(5, 10, "Doc"), client(6, 10, "Doc")]);
    for clients in [two, ready(vec![client(5, 99, "Doc")])] {
        let source = Fixed {
            clients,
            stack: snapshot(vec![window(5, false, 0, 0)]),
            reads: AtomicU32::new(0),
        };
        assert_eq!(
            frame_window_state(&source, deadline(), &facts(10, "Doc")),
            None
        );
        assert_eq!(source.reads.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn without_x11_evidence_a_window_has_no_state() {
    let nothing = [
        StackingRead::NotX11,
        StackingRead::NoManager,
        StackingRead::Unavailable,
        snapshot(vec![window(9, false, 0, 0)]),
    ];
    for stack in nothing {
        let source = Fixed {
            clients: ready(vec![client(5, 10, "Doc")]),
            stack,
            reads: AtomicU32::new(0),
        };
        assert_eq!(
            frame_window_state(&source, deadline(), &facts(10, "Doc")),
            None
        );
    }
    for clients in [X11State::Unavailable, X11State::NoWindowManager] {
        let source = Fixed {
            clients,
            stack: snapshot(vec![window(5, false, 0, 0)]),
            reads: AtomicU32::new(0),
        };
        assert_eq!(
            frame_window_state(&source, deadline(), &facts(10, "Doc")),
            None
        );
    }
}

#[test]
fn a_read_that_blocks_is_unavailable_within_the_deadline() {
    let started = Instant::now();
    let budget = Deadline::after(80).expect("a deadline builds");
    let read = bounded(budget, StackingRead::Unavailable, || {
        std::thread::sleep(Duration::from_secs(5));
        StackingRead::NoManager
    });
    assert_eq!(read, StackingRead::Unavailable);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn a_wayland_session_answers_not_x11_without_a_connection() {
    let opened = Cell::new(false);
    let read = route(DisplayServer::Wayland, |_| {
        opened.set(true);
        StackingRead::NoManager
    });
    assert_eq!(read, StackingRead::NotX11);
    assert!(!opened.get());
}

#[test]
fn a_session_with_no_display_is_unavailable_without_a_connection() {
    let opened = Cell::new(false);
    let none = Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    };
    let read = route(DisplayServer::Unavailable(none), |_| {
        opened.set(true);
        StackingRead::NoManager
    });
    assert_eq!(read, StackingRead::Unavailable);
    assert!(!opened.get());
}

#[test]
fn an_x11_session_reads_the_display_the_detector_chose() {
    let read = route(DisplayServer::X11(":7".to_owned()), |display| {
        assert_eq!(display, ":7");
        StackingRead::NoManager
    });
    assert_eq!(read, StackingRead::NoManager);
}

struct Slow {
    clients: X11State,
    stack: StackingRead,
}

impl StackingSource for Slow {
    fn read_within(&self, _target_client: u32, _deadline: Deadline) -> StackingRead {
        std::thread::sleep(Duration::from_millis(120));
        self.stack.clone()
    }

    fn clients_within(&self, _deadline: Deadline) -> X11State {
        self.clients.clone()
    }
}

#[test]
fn a_read_that_ends_after_the_callers_deadline_gives_no_state() {
    let source = Slow {
        clients: ready(vec![client(5, 10, "Doc")]),
        stack: snapshot(vec![window(5, true, 120, 80)]),
    };
    let budget = Deadline::after(60).expect("a deadline builds");
    assert_eq!(frame_window_state(&source, budget, &facts(10, "Doc")), None);
}

#[test]
fn a_read_never_waits_longer_than_the_caller_has_left() {
    let short = capped(Deadline::after(50).expect("a deadline builds")).expect("a deadline");
    assert!(short.remaining() <= Duration::from_millis(50));
    let long = capped(Deadline::after(60_000).expect("a deadline builds")).expect("a deadline");
    assert!(long.remaining() <= READ_BUDGET);
}
