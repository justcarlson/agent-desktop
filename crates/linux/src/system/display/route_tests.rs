use super::*;
use crate::system::display::display_cause::Cause;
use crate::system::display::display_unavailable::Unavailable;

fn unavailable(cause: Cause, detail: &str) -> DisplayServer {
    DisplayServer::Unavailable(Unavailable {
        cause,
        detail: detail.to_owned(),
    })
}

fn generic() -> Result<WaylandShell, AdapterError> {
    Ok(WaylandShell::Generic)
}

fn reason_of(server: DisplayServer) -> String {
    let error = resolve_with(server, generic).expect_err("an unavailable server has no route");
    error
        .details
        .as_ref()
        .and_then(|details| details.get("display_unavailable"))
        .and_then(|reason| reason.as_str())
        .map(str::to_owned)
        .expect("the refusal names why the display is unavailable")
}

#[test]
fn an_x11_answer_selects_the_x11_route_with_its_display() {
    assert_eq!(
        resolve_with(DisplayServer::X11(":7".into()), || panic!("x11 never asks"))
            .expect("x11 routes"),
        Route::X11(":7".into())
    );
}

#[test]
fn a_wayland_answer_selects_the_wayland_route_with_its_shell() {
    assert_eq!(
        resolve_with(DisplayServer::Wayland, generic).expect("wayland routes"),
        Route::Wayland(WaylandShell::Generic)
    );
    let binding = crate::system::hyprland::hyprland_binding::HyprlandBinding {
        signature: "a".into(),
        ipc_socket: "/run/user/1000/hypr/a/.socket.sock".into(),
        wayland_socket: "/run/user/1000/wayland-1".into(),
        pid: 4100,
        start_time: 7,
    };
    assert_eq!(
        resolve_with(DisplayServer::Wayland, || Ok(WaylandShell::Hyprland(
            binding.clone()
        )))
        .expect("hyprland routes"),
        Route::Wayland(WaylandShell::Hyprland(binding))
    );
}

#[test]
fn a_refused_hyprland_binding_refuses_the_wayland_route() {
    let refusal = crate::system::hyprland::bind_refusal::BindRefusal::Ambiguous {
        signatures: vec!["a".into(), "b".into()],
    };
    let error = resolve_with(DisplayServer::Wayland, || Err(refusal.to_error()))
        .expect_err("an ambiguous session has no route");
    assert_eq!(
        error.details.as_ref().expect("details")["display_unavailable"],
        "hyprland_ambiguous"
    );
}

#[test]
fn a_missing_server_and_a_refusing_one_give_different_reasons() {
    let missing = reason_of(unavailable(Cause::Missing, "DISPLAY=:9 is gone"));
    let refused = reason_of(unavailable(Cause::Refused, "DISPLAY=:9 refused"));
    assert_eq!(missing, "missing");
    assert_eq!(refused, "refused");
}

#[test]
fn no_configured_display_names_both_variables() {
    let error = resolve_with(unavailable(Cause::NotConfigured, ""), generic).expect_err("no route");
    let suggestion = error.suggestion.expect("the refusal carries a suggestion");
    assert!(suggestion.contains("DISPLAY"), "{suggestion}");
    assert!(suggestion.contains("WAYLAND_DISPLAY"), "{suggestion}");
}
