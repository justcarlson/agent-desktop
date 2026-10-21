use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;

use super::*;

fn app(pid: u32, name: &str) -> Running {
    Running {
        executable: keys(&[name]),
        pid,
        instance: format!("boot:{pid}"),
        name: name.to_owned(),
    }
}

fn keys(items: &[&str]) -> Option<super::super::executable_identity::ExecutableIdentity> {
    Some(super::super::executable_identity::ExecutableIdentity {
        executable: std::path::PathBuf::from(format!("/apps/{}", items[0])),
        argv: None,
    })
}

fn exited(code: u8) -> ExitStatus {
    ExitStatus::from_raw(i32::from(code) << 8)
}

fn killed(signal: i32) -> ExitStatus {
    ExitStatus::from_raw(signal)
}

#[test]
fn launch_observation_selects_the_owner_or_reports_the_exact_failure() {
    use Step::{Ambiguous, Failed, Owner, Wait};

    let cases = [
        (None, vec![], vec![], "tool", false, Wait),
        (
            None,
            vec![app(100, "wrapper")],
            vec![],
            "tool",
            false,
            Owner(app(100, "wrapper")),
        ),
        (None, vec![app(7, "tool")], vec![7], "tool", false, Wait),
        (
            Some(exited(0)),
            vec![app(205, "gnome-terminal-server")],
            vec![],
            "gnome-terminal-server",
            false,
            Owner(app(205, "gnome-terminal-server")),
        ),
        (
            Some(exited(0)),
            vec![app(7, "gnome-text-editor")],
            vec![7],
            "gnome-text-editor",
            false,
            Owner(app(7, "gnome-text-editor")),
        ),
        (
            Some(exited(0)),
            vec![app(7, "gnome-calculator"), app(300, "gnome-calculator")],
            vec![7],
            "gnome-calculator",
            false,
            Owner(app(300, "gnome-calculator")),
        ),
        (
            Some(exited(0)),
            vec![app(300, "tool"), app(301, "tool")],
            vec![],
            "tool",
            false,
            Ambiguous(vec![300, 301]),
        ),
        (Some(exited(0)), vec![], vec![], "tool", false, Wait),
        (
            Some(exited(0)),
            vec![],
            vec![],
            "tool",
            true,
            Failed(exited(0)),
        ),
        (
            Some(exited(3)),
            vec![],
            vec![],
            "tool",
            false,
            Failed(exited(3)),
        ),
        (
            Some(exited(2)),
            vec![app(7, "tool")],
            vec![7],
            "tool",
            true,
            Failed(exited(2)),
        ),
        (
            Some(killed(11)),
            vec![app(300, "tool")],
            vec![],
            "tool",
            true,
            Failed(killed(11)),
        ),
        (
            None,
            vec![app(7, "gnome-calculator"), app(300, "gnome-calculator")],
            vec![7],
            "gnome-calculator",
            false,
            Owner(app(300, "gnome-calculator")),
        ),
    ];

    for (index, (child, apps, before, wanted, settled, expected)) in cases.into_iter().enumerate() {
        let before = before.into_iter().collect();
        let wanted = keys(&[wanted]);
        let actual = step(&Sample {
            child_pid: 100,
            child,
            apps: &apps,
            before: &before,
            identity: &wanted,
            settled,
        });
        assert_eq!(actual, expected, "case {index}");
    }
}

#[test]
fn a_failure_names_the_status_and_marks_the_exact_crash_disposition() {
    let error = failed(killed(9), "tool");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert!(error.message.contains("signal 9"));
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.expect("details");
    assert_eq!(details["kind"], "crashed");
    assert_eq!(details["signal"], 9);

    let exited = failed(exited(4), "tool");
    assert!(exited.message.contains("status 4"));
    assert_eq!(exited.details.expect("details")["status"], 4);
}
