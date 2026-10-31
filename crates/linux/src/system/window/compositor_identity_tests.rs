use agent_desktop_core::{DeliverySemantics, ErrorCode, Rect};

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;
use crate::system::hyprland::clients::hypr_client::HyprClient;
use crate::system::hyprland::toplevel::toplevel_view::ToplevelView;

fn client(address: u64, title: &str, workspace: &str) -> HyprClient {
    HyprClient {
        address,
        pid: 7,
        title: title.into(),
        class: "editor".into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        workspace: ClientWorkspace {
            name: workspace.into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn evidence(title: &str) -> WindowEvidence {
    WindowEvidence {
        pid: 7,
        instance: Some("boot:1".into()),
        title: title.into(),
        size: None,
    }
}

#[test]
fn zero_or_several_toplevels_refuse_the_window_and_one_binds_it() {
    let clients = [
        client(0x10, "Notes", "1"),
        client(0x20, "Notes", "2"),
        client(0x30, "Preferences", "1"),
    ];
    let toplevels = [
        ToplevelView {
            identifier: "a".into(),
            address: 0x10,
        },
        ToplevelView {
            identifier: "b".into(),
            address: 0x20,
        },
        ToplevelView {
            identifier: "c".into(),
            address: 0x30,
        },
    ];
    let inventory = Inventory {
        clients: &clients,
        toplevels: &toplevels,
    };
    let live = Some("boot:1");

    let none = bound_toplevel(&evidence("Gone"), live, inventory).expect_err("no toplevel");
    let twins = bound_toplevel(&evidence("Notes"), live, inventory).expect_err("twins");
    let changed =
        bound_toplevel(&evidence("Preferences"), Some("boot:2"), inventory).expect_err("changed");

    assert_eq!(none.code, ErrorCode::WindowNotFound);
    assert_eq!(changed.code, ErrorCode::WindowNotFound);
    assert_eq!(twins.code, ErrorCode::AmbiguousTarget);
    assert_eq!(
        twins
            .details
            .as_ref()
            .map(|details| details["candidates"].clone()),
        Some(serde_json::json!([
            {"address": "0x10", "workspace": "1"},
            {"address": "0x20", "workspace": "2"},
        ]))
    );
    for error in [none, twins, changed] {
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
    assert_eq!(
        bound_toplevel(&evidence("Preferences"), live, inventory).ok(),
        Some(0x30)
    );
}
