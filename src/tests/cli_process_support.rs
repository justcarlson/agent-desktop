use std::process::Command;

pub(super) fn private_bus_binary() -> Command {
    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new("bash");
        command
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../scripts/private-session.sh"
            ))
            .args(["--", env!("CARGO_BIN_EXE_agent-desktop")]);
        command
    }
    #[cfg(not(target_os = "linux"))]
    super::binary()
}

#[cfg(target_os = "linux")]
#[test]
fn batch_without_a_bus_stops_at_permission_preflight_without_dispatching() {
    let dir = super::unique_temp_dir("cli-no-bus");
    let batch = serde_json::json!([
        {"command": "session", "args": {"action": "start", "no_trace": true}},
        {"command": "wait", "args": {"event": "window-opened", "timeout": 100}}
    ]);
    let output = super::binary()
        .args(["batch", &batch.to_string()])
        .env("AGENT_DESKTOP_HOME", &dir)
        .env("XDG_RUNTIME_DIR", &dir)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            format!("unix:path={}/absent-bus", dir.display()),
        )
        .env_remove("AT_SPI_BUS_ADDRESS")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
        .expect("batch runs without a bus");
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{response}");
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], "PERM_DENIED", "{response}");
    assert_eq!(response["error"]["details"]["batch_index"], 1, "{response}");
    assert_eq!(
        response["error"]["details"]["batch_command"], "wait",
        "{response}"
    );
    assert!(
        response.get("data").is_none(),
        "no entries dispatched: {response}"
    );
    assert!(
        !dir.join("sessions").exists(),
        "preflight writes no session"
    );
    let _ = std::fs::remove_dir_all(dir);
}
