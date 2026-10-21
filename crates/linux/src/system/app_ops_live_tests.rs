use agent_desktop_core::{Deadline, ObservationOps, SystemOps, launch_options::LaunchOptions};

use crate::LinuxAdapter;

#[test]
#[ignore = "launches desktop apps; run through scripts/private-session.sh --wm"]
fn live_desktop_ids_round_trip_through_launch_inventory_selection_and_close() {
    assert!(std::env::var_os("AGENT_DESKTOP_PRIVATE_SESSION").is_some());
    let adapter = LinuxAdapter::new();
    for id in ["org.gnome.Calculator", "org.gnome.Nautilus"] {
        let lease = adapter
            .acquire_interaction_lease(Deadline::after(30_000).unwrap())
            .unwrap();
        let launched = adapter
            .launch_app(id, &LaunchOptions::default(), &lease)
            .expect("launch by desktop id");
        let apps = adapter
            .list_apps_scoped("", Some(id), lease.deadline())
            .expect("select by bundle id");
        assert_eq!(apps.len(), 1, "one app for {id}");
        let app = &apps[0];
        assert_eq!(app.pid, launched.pid);
        assert_eq!(app.bundle_id.as_deref(), Some(id));
        let by_name = adapter
            .list_apps_scoped(id, None, lease.deadline())
            .expect("same identifier as name");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].pid, app.pid);
        assert_eq!(by_name[0].bundle_id, app.bundle_id);
        let executable = std::fs::read_link(format!("/proc/{}/exe", app.pid))
            .expect("independent process oracle");
        let expected = if id == "org.gnome.Calculator" {
            "gnome-calculator"
        } else {
            "nautilus"
        };
        assert_eq!(executable.file_name().unwrap(), expected);
        adapter
            .close_app(app, false, &lease)
            .expect("close selected desktop id");
        match std::fs::read_to_string(format!("/proc/{}/stat", app.pid)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(stat) => assert!(
                stat.rsplit_once(") ")
                    .is_some_and(|(_, tail)| matches!(tail.chars().next(), Some('Z' | 'X'))),
                "close leaves no running process"
            ),
            Err(error) => panic!("process exit oracle: {error}"),
        }
    }
}
