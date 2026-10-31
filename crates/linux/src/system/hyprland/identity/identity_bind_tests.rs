use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

fn client(address: u64, pid: u32, title: &str, size: (f64, f64), workspace: &str) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: title.into(),
        class: "app".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: size.0,
            height: size.1,
        },
        workspace: ClientWorkspace {
            name: workspace.into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn view(identifier: &str, address: u64) -> ToplevelView {
    ToplevelView {
        identifier: identifier.into(),
        address,
        activated: false,
        wlr_index: Some(0),
    }
}

fn frame(pid: u32, title: &str, size: Option<(f64, f64)>) -> WindowEvidence {
    WindowEvidence {
        pid,
        instance: Some("boot:100".into()),
        title: title.into(),
        size,
    }
}

const LIVE: Option<&str> = Some("boot:100");

#[test]
fn a_frame_binds_to_the_one_client_of_its_process_with_its_exact_title() {
    let clients = [
        client(0x10, 7, "Notes", (400.0, 300.0), "1"),
        client(0x20, 7, "Notes — draft", (400.0, 300.0), "1"),
        client(0x30, 8, "Notes", (400.0, 300.0), "1"),
    ];
    let toplevels = [view("a", 0x10), view("b", 0x20), view("c", 0x30)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let bound = bind_frame(&frame(7, "Notes", Some((1.0, 1.0))), LIVE, inventory).expect("bound");

    assert_eq!(bound.client.address, 0x10);
    assert_eq!(bound.toplevel.identifier, "a");
}

#[test]
fn a_frame_whose_process_restarted_or_is_unknown_is_refused() {
    let clients = [client(0x10, 7, "Notes", (1.0, 1.0), "1")];
    let toplevels = [view("a", 0x10)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let mut unknown = frame(7, "Notes", None);
    unknown.instance = None;

    assert_eq!(
        bind_frame(&frame(7, "Notes", None), Some("boot:101"), inventory),
        Err(IdentityRefusal::ProcessChanged)
    );
    assert_eq!(
        bind_frame(&frame(7, "Notes", None), None, inventory),
        Err(IdentityRefusal::ProcessChanged)
    );
    assert_eq!(
        bind_frame(&unknown, LIVE, inventory),
        Err(IdentityRefusal::ProcessChanged)
    );
}

#[test]
fn a_frame_with_no_matching_client_or_handle_has_no_candidate() {
    let clients = [
        client(0x10, 7, "Other", (1.0, 1.0), "1"),
        client(0x20, 7, "Notes", (1.0, 1.0), "1"),
    ];
    let toplevels = [view("a", 0x10)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    assert_eq!(
        bind_frame(&frame(7, "Notes", None), LIVE, inventory),
        Err(IdentityRefusal::NoCandidate)
    );
}

#[test]
fn same_titled_twins_are_told_apart_by_size_only_when_exactly_one_size_matches() {
    let clients = [
        client(0x10, 7, "Twin", (400.0, 300.0), "1"),
        client(0x20, 7, "Twin", (621.0, 750.0), "1"),
    ];
    let toplevels = [view("a", 0x10), view("b", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let bound = bind_frame(&frame(7, "Twin", Some((621.0, 750.0))), LIVE, inventory);

    assert_eq!(bound.map(|bound| bound.client.address), Ok(0x20));
}

#[test]
fn twins_that_size_cannot_separate_are_refused_with_every_candidate_named() {
    let clients = [
        client(0x20, 7, "Twin", (621.0, 750.0), "3"),
        client(0x10, 7, "Twin", (621.0, 750.0), "1"),
    ];
    let toplevels = [view("a", 0x10), view("b", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let expected = Err(IdentityRefusal::Ambiguous(vec![
        (0x10, "1".into()),
        (0x20, "3".into()),
    ]));

    assert_eq!(
        bind_frame(&frame(7, "Twin", Some((621.0, 750.0))), LIVE, inventory),
        expected
    );
    assert_eq!(
        bind_frame(&frame(7, "Twin", None), LIVE, inventory),
        expected
    );
    assert_eq!(
        bind_frame(&frame(7, "Twin", Some((10.0, 10.0))), LIVE, inventory),
        expected
    );
}

#[test]
fn a_sole_title_match_binds_even_when_tiling_changed_its_size() {
    let clients = [client(0x10, 7, "Calculator", (70.0, 85.0), "1")];
    let toplevels = [view("a", 0x10)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };

    let bound = bind_frame(
        &frame(7, "Calculator", Some((360.0, 616.0))),
        LIVE,
        inventory,
    );

    assert_eq!(bound.map(|bound| bound.client.address), Ok(0x10));
}

#[test]
fn a_compositor_window_binds_by_its_identifier_and_owner() {
    let clients = [
        client(0x10, 7, "foot", (1.0, 1.0), "1"),
        client(0x20, 7, "foot", (1.0, 1.0), "1"),
    ];
    let toplevels = [view("a", 0x10), view("b", 0x20)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let id = CompositorWindowId {
        compositor_pid: 50,
        compositor_start: 60,
        identifier: "b".into(),
    };

    let bound = bind_compositor(&id, &frame(7, "renamed", None), (50, 60), LIVE, inventory);

    assert_eq!(bound.map(|bound| bound.client.address), Ok(0x20));
}

#[test]
fn a_compositor_window_from_another_compositor_or_owner_or_gone_is_refused() {
    let clients = [client(0x10, 7, "foot", (1.0, 1.0), "1")];
    let toplevels = [view("a", 0x10), view("orphan", 0x99)];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let id = |identifier: &str| CompositorWindowId {
        compositor_pid: 50,
        compositor_start: 60,
        identifier: identifier.into(),
    };
    let owner = frame(7, "foot", None);

    assert_eq!(
        bind_compositor(&id("a"), &owner, (50, 61), LIVE, inventory),
        Err(IdentityRefusal::OtherCompositor)
    );
    assert_eq!(
        bind_compositor(&id("a"), &owner, (50, 60), Some("boot:9"), inventory),
        Err(IdentityRefusal::ProcessChanged)
    );
    assert_eq!(
        bind_compositor(&id("a"), &frame(8, "foot", None), (50, 60), LIVE, inventory),
        Err(IdentityRefusal::ProcessChanged)
    );
    assert_eq!(
        bind_compositor(&id("gone"), &owner, (50, 60), LIVE, inventory),
        Err(IdentityRefusal::ToplevelGone)
    );
    assert_eq!(
        bind_compositor(&id("orphan"), &owner, (50, 60), LIVE, inventory),
        Err(IdentityRefusal::ToplevelGone)
    );
}
