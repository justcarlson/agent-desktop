use agent_desktop_core::{
    EventKind, SignalBaseline, SignalCompleteness, SnapshotSurface, WindowInfo, WindowState,
    diff_signals,
};

use super::{SIGNAL_SURFACE_KINDS, sheet_signals};
use crate::system::listed_window::ListedWindow;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry_app::{RegistryApp, assemble};
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::FrameFacts;
use crate::tree::AtspiElement;

fn app() -> RegistryApp {
    assemble(
        AtspiElement::new(":1.4", "/org/a11y/atspi/accessible/root", 77),
        ProcessFacts {
            comm: "calc".to_owned(),
            exe_basename: Some("calc".to_owned()),
            instance: "boot:77".to_owned(),
            web_family: None,
        },
        Some(Some("Calculator".to_owned())),
        ToolkitId::default(),
    )
}

fn window(id: &str, title: &str, sheet: bool) -> ListedWindow {
    ListedWindow {
        app: 0,
        info: WindowInfo {
            id: id.to_owned(),
            title: title.to_owned(),
            app: "calc".to_owned(),
            pid: 77.into(),
            process_instance: Some("boot:77".to_owned()),
            bounds: None,
            state: WindowState {
                is_focused: false,
                accessible: true,
                minimized: None,
                visible: Some(true),
            },
        },
        active: false,
        frame: FrameFacts {
            pid: 77,
            title: title.to_owned(),
            extents: None,
            global_position: false,
        },
        sheet,
        unshown: false,
    }
}

fn baseline(windows: &[ListedWindow]) -> SignalBaseline {
    SignalBaseline {
        windows: windows.iter().map(|window| window.info.clone()).collect(),
        apps: Vec::new(),
        surfaces: sheet_signals(windows, &[app()]),
        completeness: SignalCompleteness::complete(),
    }
}

#[test]
fn only_sheet_windows_become_signals_and_each_carries_its_identity() {
    let windows = [
        window("w-1", "Calculator", false),
        window("w-2", "About", true),
    ];
    let signals = sheet_signals(&windows, &[app()]);
    let [signal] = signals.as_slice() else {
        panic!("one signal")
    };
    assert_eq!(signal.kind, SnapshotSurface::Sheet);
    assert_eq!(signal.id, "w-2");
    assert_eq!(signal.title.as_deref(), Some("About"));
    assert_eq!(signal.app, "calc");
    assert_eq!(signal.pid.get(), 77);
    assert_eq!(signal.process_instance, "boot:77");
}

#[test]
fn a_dialog_without_a_name_is_keyed_on_its_id() {
    let signals = sheet_signals(&[window("w-3", "", true)], &[app()]);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].id, "w-3");
    assert_eq!(signals[0].title, None);
}

#[test]
fn every_emitted_kind_is_an_advertised_kind() {
    assert_eq!(SIGNAL_SURFACE_KINDS, [SnapshotSurface::Sheet]);
    let signals = sheet_signals(&[window("w-2", "About", true)], &[app()]);
    assert!(
        signals
            .iter()
            .all(|signal| SIGNAL_SURFACE_KINDS.contains(&signal.kind))
    );
}

#[test]
fn a_dialog_listed_as_window_and_sheet_is_one_window_opened_and_one_surface() {
    let before = baseline(&[window("w-1", "Calculator", false)]);
    let after = baseline(&[
        window("w-1", "Calculator", false),
        window("w-2", "About", true),
    ]);
    let events = diff_signals(&before, &after);
    let opened = events
        .iter()
        .filter(|event| event.kind == EventKind::WindowOpened)
        .count();
    let appeared = events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                EventKind::SurfaceAppeared {
                    surface: SnapshotSurface::Sheet
                }
            )
        })
        .count();
    assert_eq!(opened, 1);
    assert_eq!(appeared, 1);
    assert_eq!(events.len(), 2);
}

#[test]
fn a_dialog_that_closes_is_one_window_closed_and_one_surface_dismissed() {
    let before = baseline(&[window("w-2", "About", true)]);
    let after = baseline(&[]);
    let events = diff_signals(&before, &after);
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::SurfaceDismissed { .. }))
    );
}

#[test]
fn a_window_naming_a_missing_registry_entry_gives_no_signal() {
    assert!(sheet_signals(&[window("w-2", "About", true)], &[]).is_empty());
}

fn nested(id: &str, title: Option<&str>) -> crate::system::nested_scan::Nested {
    crate::system::nested_scan::Nested {
        window: 0,
        id: id.to_owned(),
        title: title.map(str::to_owned),
    }
}

fn scan(found: Vec<crate::system::nested_scan::Nested>) -> crate::system::nested_scan::NestedScan {
    crate::system::nested_scan::NestedScan {
        found,
        complete: true,
        unread: Vec::new(),
    }
}

#[test]
fn a_dialog_inside_a_window_is_one_sheet_signal_on_its_own_id() {
    let windows = [window("w-1.4-win", "Editor", false)];
    let signals = super::nested_signals(
        &scan(vec![nested("w-1.4-dlg", Some("About"))]),
        &windows,
        &[app()],
    );
    let [signal] = signals.as_slice() else {
        panic!("one signal")
    };
    assert_eq!(signal.id, "w-1.4-dlg");
    assert_eq!(signal.kind, SnapshotSurface::Sheet);
    assert_eq!(signal.pid.get(), 77);
    assert_eq!(signal.title.as_deref(), Some("About"));
    assert!(SIGNAL_SURFACE_KINDS.contains(&signal.kind));
}

#[test]
fn a_nested_dialog_is_announced_when_it_shows_and_dismissed_when_it_no_longer_does() {
    let windows = [window("w-1.4-win", "Editor", false)];
    let with = |found| SignalBaseline {
        windows: Vec::new(),
        apps: Vec::new(),
        surfaces: super::nested_signals(&scan(found), &windows, &[app()]),
        completeness: SignalCompleteness::complete(),
    };
    let closed = with(Vec::new());
    let open = with(vec![nested("w-1.4-dlg", None)]);
    let appeared = diff_signals(&closed, &open);
    assert_eq!(appeared.len(), 1);
    assert_eq!(
        appeared[0].kind,
        EventKind::SurfaceAppeared {
            surface: SnapshotSurface::Sheet
        }
    );
    let dismissed = diff_signals(&open, &closed);
    assert_eq!(dismissed.len(), 1);
    assert_eq!(
        dismissed[0].kind,
        EventKind::SurfaceDismissed {
            surface: SnapshotSurface::Sheet
        }
    );
}
