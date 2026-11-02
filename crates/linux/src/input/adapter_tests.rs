use agent_desktop_core::{Action, ActionRequest, AdapterError, Direction, InputOps, RefEntry};

use crate::adapter::LinuxAdapter;

const BOUND_CHILD: &str = "AD_INPUT_ADAPTER_BOUND_CHILD";

fn run_on_bound_hyprland(test: &str) -> bool {
    run_on_wayland(test, true)
}

fn run_on_wayland(test: &str, bound: bool) -> bool {
    if std::env::var_os(BOUND_CHILD).is_some() {
        return false;
    }
    let tag = {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        test.hash(&mut hasher);
        hasher.finish()
    };
    let dir = std::env::temp_dir().join(format!("ad-ref-refusal-{}-{tag:x}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a socket directory");
    let socket = dir.join("wayland-refusal");
    let _ = std::fs::remove_file(&socket);
    let listener = std::os::unix::net::UnixListener::bind(&socket).expect("a listening socket");
    let mut command = std::process::Command::new(std::env::current_exe().expect("test binary"));
    command
        .args([
            "--exact",
            &format!("input::adapter::tests::{test}"),
            "--nocapture",
        ])
        .env(BOUND_CHILD, "1")
        .env("WAYLAND_DISPLAY", &socket)
        .env("XDG_RUNTIME_DIR", &dir)
        .env("DBUS_SESSION_BUS_ADDRESS", "unix:path=/nonexistent")
        .env_remove("AT_SPI_BUS_ADDRESS")
        .env_remove("DISPLAY")
        .env_remove("HYPRLAND_INSTANCE_SIGNATURE");
    if bound {
        command.env(crate::input::hyprland::hypr_route::TEST_ROUTE_VARIABLE, "1");
    } else {
        command.env_remove(crate::input::hyprland::hypr_route::TEST_ROUTE_VARIABLE);
    }
    let output = command.output().expect("isolated test starts");
    drop(listener);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "{test} must execute exactly one test: {output:?}"
    );
    true
}

fn unresolvable_entry_in_a_valid_window() -> RefEntry {
    let crate::system::process::process_lookup::Lookup::Found(facts) =
        crate::system::process::process_facts::lookup(std::process::id())
    else {
        panic!("the test process reads its own facts");
    };
    serde_json::from_value(serde_json::json!({
        "pid": std::process::id(), "process_instance": facts.instance, "role": "button",
        "name": "Run", "source_window_id": "w-1.30-org-a11y-atspi-accessible-1",
        "source_surface": "window", "path": [0], "available_actions": ["Click"], "states": []
    }))
    .expect("a ref entry")
}

fn assert_refused_before_delivery(error: &AdapterError, label: &str) {
    let details = error.details.clone().unwrap_or_default();
    assert_eq!(details["cause"], "receiver_unproven", "{label}: {error:?}");
    assert_eq!(details["physical_delivery_started"], false, "{label}");
    assert_eq!(
        serde_json::to_value(error.disposition).expect("disposition")["delivery"],
        "not_delivered",
        "{label}"
    );
}

#[test]
fn a_bound_hyprland_refuses_a_ref_it_cannot_resolve_before_any_lease_or_activation() {
    if run_on_bound_hyprland(
        "a_bound_hyprland_refuses_a_ref_it_cannot_resolve_before_any_lease_or_activation",
    ) {
        return;
    }
    let adapter = LinuxAdapter::new();
    let entry = unresolvable_entry_in_a_valid_window();
    for timeout_ms in [Some(0), None, Some(2_000)] {
        for action in [
            Action::Click,
            Action::DoubleClick,
            Action::TripleClick,
            Action::RightClick,
            Action::Scroll(Direction::Down, 3),
        ] {
            let label = format!("{} timeout {timeout_ms:?}", action.name());
            let request = ActionRequest::headed(action).with_timeout_ms(timeout_ms);
            let started = std::time::Instant::now();

            let error = agent_desktop_core::ref_action::execute_entry(&adapter, &entry, request)
                .expect_err("a bound Hyprland delivers no headed ref pointer action");

            assert_refused_before_delivery(&error, &label);
            assert!(
                started.elapsed() < std::time::Duration::from_millis(1_000),
                "{label} waited before it refused"
            );
        }
    }
}

#[test]
fn a_bound_hyprland_activates_the_window_before_a_headed_key_by_ref() {
    if run_on_bound_hyprland("a_bound_hyprland_activates_the_window_before_a_headed_key_by_ref") {
        return;
    }
    assert!(
        LinuxAdapter::new().keyboard_needs_window_activation(),
        "a GTK GrabFocus present is an activation request the compositor may refuse (L48-1)"
    );
}

#[test]
fn an_unbound_wayland_session_leaves_activation_to_the_focus_gate() {
    if run_on_wayland(
        "an_unbound_wayland_session_leaves_activation_to_the_focus_gate",
        false,
    ) {
        return;
    }
    assert!(!LinuxAdapter::new().keyboard_needs_window_activation());
}
