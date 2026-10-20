use super::*;
use agent_desktop_core::{AppError, CommandContext, ErrorCode, SnapshotSurface, SystemOps};

use super::*;
use agent_desktop_core::{AppError, CommandContext, ErrorCode, SnapshotSurface, SystemOps};

fn snapshot_args(surface: SnapshotSurface) -> agent_desktop_core::commands::snapshot::SnapshotArgs {
    agent_desktop_core::commands::snapshot::SnapshotArgs {
        app: None,
        window_id: None,
        max_depth: 1,
        include_bounds: false,
        interactive_only: false,
        compact: true,
        surface,
        skeleton: false,
        root_ref: None,
        timeout_ms: None,
        force_electron_a11y: false,
    }
}

#[test]
fn supported_surfaces_advertises_window_and_sheet_and_rejects_the_rest() {
    let adapter = LinuxAdapter::new();
    assert_eq!(
        adapter.supported_surfaces(),
        vec![SnapshotSurface::Window, SnapshotSurface::Sheet]
    );

    let error = agent_desktop_core::commands::snapshot::execute(
        snapshot_args(SnapshotSurface::Menu),
        &adapter,
        &CommandContext::default(),
    )
    .expect_err("an unsupported surface must fail at validation");

    let AppError::Adapter(error) = error else {
        panic!("surface validation must return an adapter error")
    };
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("supported_surfaces"))
            .and_then(|surfaces| surfaces.as_array())
            .map(Vec::len),
        Some(2)
    );
}

#[test]
fn a_window_surface_passes_validation_and_reaches_the_adapter() {
    let adapter = LinuxAdapter::new();
    let error = agent_desktop_core::commands::snapshot::execute(
        snapshot_args(SnapshotSurface::Window),
        &adapter,
        &CommandContext::default(),
    )
    .expect_err("no session can answer in a unit test");

    let AppError::Adapter(error) = error else {
        panic!("the observation must fail as an adapter error")
    };
    assert!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("requested_surface"))
            .is_none(),
        "the failure must come from the adapter, not surface validation: {error:?}"
    );
}

#[test]
fn every_live_read_is_the_adapters_own_and_not_the_unsupported_default() {
    let adapter = LinuxAdapter::new();
    let foreign = agent_desktop_core::NativeHandle::new(5u32);
    let deadline = agent_desktop_core::Deadline::after(1_000).expect("a deadline builds");
    let errors = [
        adapter.get_live_value(&foreign, deadline).err(),
        adapter.get_live_state(&foreign, deadline).err(),
        adapter.get_live_actions(&foreign, deadline).err(),
        adapter.get_live_element(&foreign, deadline).err(),
        adapter.get_element_bounds(&foreign, deadline).err(),
    ];
    for error in errors {
        let error = error.expect("a foreign handle is refused");
        assert_eq!(error.code, ErrorCode::InvalidArgs, "{error:?}");
    }
}

fn lease() -> InteractionLease {
    InteractionLease::guarded(Deadline::after(2_000).expect("a deadline builds"), ())
        .expect("a lease")
}

fn not_the_default(error: &AdapterError) {
    assert_ne!(
        error.code,
        ErrorCode::PlatformNotSupported,
        "the adapter answers for itself: {error:?}"
    );
}

#[test]
fn the_lifecycle_overrides_are_wired_to_the_adapter() {
    let adapter = LinuxAdapter::new();
    assert!(adapter.is_protected_process("gnome-shell"));
    assert!(!adapter.is_protected_process("gnome-text-editor"));

    let gone = std::process::Command::new("true")
        .spawn()
        .and_then(|mut child| child.wait().map(|_| child.id()))
        .expect("a child that exits");
    let identity = ProcessIdentity::new(gone, "deadbeef:1");
    let deadline = Deadline::after(2_000).expect("a deadline builds");
    assert_eq!(
        adapter
            .process_state(identity.clone(), deadline)
            .expect("an exited process has a state"),
        agent_desktop_core::process_state::ProcessState::Exited { code: None }
    );

    let protected = AppInfo {
        name: "gnome-shell".to_owned(),
        pid: gone.into(),
        bundle_id: None,
        process_instance: Some("deadbeef:1".to_owned()),
        presentation: None,
    };
    let error = adapter
        .close_app(&protected, false, &lease())
        .expect_err("a protected name is refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);

    let mut options = agent_desktop_core::launch_options::LaunchOptions::default();
    options.args.push("a\0b".to_owned());
    let error = adapter
        .launch_app("anything", &options, &lease())
        .expect_err("a NUL argument is refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);

    let window = agent_desktop_core::WindowInfo {
        id: "w-1.2-a".to_owned(),
        title: String::new(),
        app: "app".to_owned(),
        pid: gone.into(),
        process_instance: None,
        bounds: None,
        state: agent_desktop_core::WindowState::default(),
    };
    let error = adapter
        .window_op(&window, agent_desktop_core::WindowOp::Minimize, &lease())
        .expect_err("a window without its process instance is refused");
    not_the_default(&error);

    let combo = KeyCombo {
        key: "n".to_owned(),
        modifiers: Vec::new(),
    };
    let error = adapter
        .press_key_for_app(
            identity,
            &combo,
            agent_desktop_core::InteractionPolicy::headless(),
            &lease(),
        )
        .expect_err("no desktop here");
    not_the_default(&error);
}
