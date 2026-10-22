use std::process::Command;

use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode};

use super::anchor::Anchor;
use super::container::{Landmark, locate_anchor};
use super::identity::find_root;
use crate::system::SharedBus;

#[test]
#[ignore = "private native Wayland shell; run through scripts/private-session.sh"]
fn unproven_wayland_popup_openers_refuse_without_pointer_delivery() {
    assert!(std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some());
    if std::env::var_os("AD_SHELL_WAYLAND_CHILD").is_none() {
        let status = Command::new("bash")
            .args(["scripts/private-wayland-shell.sh", "--no-input", "--"])
            .arg(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "system::shell::live_wayland_tests::unproven_wayland_popup_openers_refuse_without_pointer_delivery", "--nocapture"])
            .env("AD_SHELL_WAYLAND_CHILD", "1")
            .status().unwrap();
        assert!(status.success(), "private Wayland child");
        return;
    }
    let bus = SharedBus::default();
    let root = find_root(&bus, Deadline::after(10_000).unwrap()).expect("native shell root");
    super::owner_live_tests::shell_identity_follows_both_bus_owners_and_never_the_process_name();
    for anchor in [
        Anchor::Landmark(Landmark::DateMenu),
        Anchor::SystemButton,
        Anchor::Landmark(Landmark::SystemMenu),
    ] {
        let error = locate_anchor(&bus, &root, anchor, Deadline::after(5_000).unwrap())
            .expect_err("labels and position cannot identify a popup opener");
        assert_eq!(error.code, ErrorCode::ActionNotSupported);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    }
    let oracle = Command::new("busctl")
        .args([
            "--user",
            "call",
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "GetConnectionUnixProcessID",
            "s",
            "org.gnome.Shell",
        ])
        .output()
        .unwrap();
    assert!(oracle.status.success());
    assert_eq!(
        String::from_utf8(oracle.stdout).unwrap().trim(),
        format!("u {}", root.pid)
    );
    println!(
        "busctl confirms native Wayland shell owner {}; unproven openers refuse without input",
        root.pid
    );
}
