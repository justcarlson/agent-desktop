use agent_desktop_core::Rect;

use super::*;
use crate::system::hyprland::clients::client_workspace::ClientWorkspace;

fn client(address: u64, pid: u32, class: &str, title: &str) -> HyprClient {
    HyprClient {
        address,
        pid,
        title: title.into(),
        class: class.into(),
        geometry: Rect {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        workspace: ClientWorkspace {
            name: "1".into(),
            monitor: Some(0),
        },
        shown: true,
    }
}

fn reads(first: Vec<HyprClient>, second: Vec<HyprClient>) -> Vec<HyprClient> {
    let mut answers = vec![second, first];
    let toplevels_read = std::cell::Cell::new(false);
    let (clients, _) = stable_inventory(
        || {
            assert_eq!(
                answers.len() == 1,
                toplevels_read.get(),
                "toplevels read between"
            );
            Ok(answers.pop().unwrap_or_default())
        },
        || {
            toplevels_read.set(true);
            Ok(Vec::<ToplevelView>::new())
        },
    )
    .expect("read");
    clients
}

#[test]
fn an_address_reused_by_another_window_across_the_toplevel_read_is_dropped() {
    let kept: Vec<u64> = reads(
        vec![
            client(0x10, 7, "foot", "shell"),
            client(0x20, 8, "firefox", "Docs"),
        ],
        vec![
            client(0x10, 9, "imv", "photo.png"),
            client(0x20, 8, "firefox", "Docs"),
        ],
    )
    .iter()
    .map(|client| client.address)
    .collect();

    assert_eq!(kept, vec![0x20]);
}

#[test]
fn a_retitled_window_is_kept_with_its_newer_title() {
    let kept = reads(
        vec![client(0x10, 7, "foot", "shell")],
        vec![client(0x10, 7, "foot", "shell: make")],
    );

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].title, "shell: make");
}
