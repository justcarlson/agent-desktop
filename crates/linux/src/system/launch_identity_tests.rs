use super::tests::{fixture, keys_in};
use crate::system::launch_attach::{Decision, decide, identity_keys};
use crate::system::running_app::Running;

fn running(name: &str) -> Running {
    Running {
        pid: 42,
        instance: "boot:42".to_owned(),
        name: name.to_owned(),
        registry_name: Some(name.to_owned()),
        comm: name.to_owned(),
    }
}

#[test]
fn a_shared_exec_cannot_attach_to_another_desktop_apps_interpreter() {
    let fixture = fixture(
        &[
            ("one", "Name=One\nType=Application\nExec=sh /opt/one.sh"),
            ("two", "Name=Two\nType=Application\nExec=sh /opt/two.sh"),
        ],
        &["sh"],
    );
    for id in ["one", "two"] {
        let keys = identity_keys(&keys_in(&fixture, id).expect("desktop launch"));
        assert_eq!(
            decide(&keys, &[running("sh")], true, false),
            Decision::Spawn
        );
        assert_eq!(
            decide(&keys, &[running(id)], true, false),
            Decision::Attach(running(id))
        );
    }
}

#[test]
fn try_exec_checks_availability_without_attaching_to_the_helper() {
    let fixture = fixture(
        &[(
            "client",
            "Name=Client\nType=Application\nExec=target\nTryExec=helper",
        )],
        &["target", "helper"],
    );
    let keys = identity_keys(&keys_in(&fixture, "client").expect("installed helper"));
    assert_eq!(
        decide(&keys, &[running("helper")], true, false),
        Decision::Spawn
    );
    assert_eq!(
        decide(&keys, &[running("target")], true, false),
        Decision::Attach(running("target"))
    );
    std::fs::remove_file(fixture.search.path_dirs[0].join("helper")).expect("remove helper");
    assert_eq!(
        keys_in(&fixture, "client"),
        Err(agent_desktop_core::ErrorCode::AppNotFound)
    );
}
