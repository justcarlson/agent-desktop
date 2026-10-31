use super::*;

#[test]
fn a_permanent_display_cause_precedes_bus_resolution() {
    const CHILD: &str = "AD_RESOLVE_DISPLAY_CHILD";
    const FILTER: &str =
        "tree::resolve::resolve_source::tests::a_permanent_display_cause_precedes_bus_resolution";
    if std::env::var_os(CHILD).is_some() {
        let bus = SharedBus::default();
        let error = match BusSource::open(&bus, Deadline::after(1_000).expect("deadline")) {
            Ok(_) => panic!("no display must refuse resolution"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            agent_desktop_core::ErrorCode::PlatformNotSupported
        );
        assert_eq!(
            error.details.expect("cause")["display_unavailable"],
            "not_configured"
        );
        return;
    }
    let runtime = std::env::current_dir()
        .expect("test directory")
        .join(format!(".resolve-no-session-{}", std::process::id()));
    std::fs::create_dir(&runtime).expect("empty runtime directory");
    let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", FILTER, "--nocapture"])
        .env(CHILD, "1")
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("HYPRLAND_INSTANCE_SIGNATURE")
        .env("XDG_RUNTIME_DIR", &runtime)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent-ad-resolve-bus",
        )
        .output()
        .expect("isolated environment");
    std::fs::remove_dir(&runtime).expect("runtime directory removed");
    assert!(output.status.success(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "{FILTER} must execute exactly one test: {output:?}"
    );
}
